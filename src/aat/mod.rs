mod types;
mod equality;
mod schema;
mod validation;

pub use types::*;
use anyhow::{Result, bail};
use schemars::Schema;
use serde_json::Value;
use equality::types_are_structurally_equal;
use schema::schema_to_type;
use validation::{validate_references, validate_path_parameter_type};

/* Abstract API Tree */
#[derive(Debug, Clone)]
pub struct AAT {
    pub types: Vec<NamedType>,
    pub services: Vec<Service>,
    type_names: std::collections::HashSet<String>,
}

impl AAT {
    pub fn new() -> Self {
        Self {
            types: vec![],
            services: vec![],
            type_names: std::collections::HashSet::new(),
        }
    }

    /// Validates that all type references in the AAT resolve to actual types
    pub fn validate(&self) -> Result<()> {
        validate_references(&self.services, &self.types)
    }

    pub fn from_spec(spec: &crate::spec::Spec) -> Result<Self> {
        let mut aat = Self::new();
        aat.import_from_spec(spec)?;
        Ok(aat)
    }

    pub fn import_from_spec(&mut self, spec: &crate::spec::Spec) -> Result<()> {
        use crate::spec::PathSegment as SpecPathSegment;

        // Iterate over services
        for spec_service in spec.services() {
            let mut aat_service = Service {
                name: spec_service.name().to_string(),
                endpoints: vec![],
            };

            // Iterate over endpoints
            for spec_endpoint in spec_service.endpoints() {
                // Extract schemas from path segments and convert path
                let mut aat_path = Vec::new();
                for segment in spec_endpoint.path() {
                    match segment {
                        SpecPathSegment::Literal(lit) => {
                            aat_path.push(PathSegment::Literal(lit.clone()));
                        }
                        SpecPathSegment::Type { name, r#type } => {
                            // Validate that path parameter type is simple
                            validate_path_parameter_type(r#type)?;
                            let field_type = self.spec_type_to_field_type(r#type)?;
                            aat_path.push(PathSegment::Parameter {
                                name: name.clone(),
                                r#type: field_type,
                            });
                        }
                    }
                }

                // Extract schema from query type
                let query_field_type = if let Some(query_type) = spec_endpoint.query_type() {
                    Some(self.spec_type_to_field_type(query_type)?)
                } else {
                    None
                };

                // Extract schema from body type
                let body_field_type = if let Some(body_type) = spec_endpoint.body_type() {
                    Some(self.spec_type_to_field_type(body_type)?)
                } else {
                    None
                };

                // Extract schema from response type
                let response_field_type =
                    self.spec_type_to_field_type(spec_endpoint.response_type())?;

                // Convert HTTP method
                let method = match spec_endpoint.method() {
                    crate::spec::Method::Get => HttpMethod::Get,
                    crate::spec::Method::Post => HttpMethod::Post,
                    crate::spec::Method::Put => HttpMethod::Put,
                    crate::spec::Method::Delete => HttpMethod::Delete,
                    crate::spec::Method::Patch => HttpMethod::Patch,
                };

                // Create AAT endpoint
                let aat_endpoint = Endpoint {
                    name: spec_endpoint.name().to_string(),
                    method,
                    path: aat_path,
                    query: query_field_type,
                    body: body_field_type,
                    response: response_field_type,
                };

                aat_service.endpoints.push(aat_endpoint);
            }

            self.services.push(aat_service);
        }

        Ok(())
    }

    fn spec_type_to_field_type(&mut self, r#type: &crate::spec::Type) -> Result<FieldType> {
        use crate::spec::Type;

        match r#type {
            Type::Void => Ok(FieldType::Any),
            Type::Schema(schema) => {
                let name = self.add_schema_and_get_name(schema)?;
                Ok(FieldType::Reference(name))
            }
            Type::List(inner) => {
                let inner_type = self.spec_type_to_field_type(inner)?;
                Ok(FieldType::List(Box::new(inner_type)))
            }
            Type::Optional(inner) => {
                let inner_type = self.spec_type_to_field_type(inner)?;
                Ok(FieldType::Optional(Box::new(inner_type)))
            }
            Type::Stream(_) => {
                bail!("Stream types are not yet fully supported in AAT conversion. Consider using a List type or implementing streaming at the transport layer.")
            }
            Type::Tuple(_) => {
                bail!("Tuple types are not yet fully supported in AAT conversion. Consider using a named struct type instead.")
            }
            Type::NamedTuple(_) => {
                bail!("NamedTuple types are not yet fully supported in AAT conversion. Consider using a named struct type instead.")
            }
        }
    }

    pub fn append_types_from_schema(&mut self, schema: &Schema, root_name: &str) -> Result<()> {
        // Convert the root schema itself
        let root_type = schema_to_type(schema, root_name)?;
        self.add_type_with_dedup_check(root_type)?;

        // Process definitions if present
        if let Some(obj) = schema.as_object() {
            if let Some(Value::Object(defs)) = obj.get("$defs").or_else(|| obj.get("definitions")) {
                for (name, def_value) in defs {
                    // Convert Value to Schema
                    if let Ok(def_schema) = Schema::try_from(def_value.clone()) {
                        let def_type = schema_to_type(&def_schema, name)?;
                        self.add_type_with_dedup_check(def_type)?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Adds a type to the AAT, checking for duplicate names with different structures
    fn add_type_with_dedup_check(&mut self, new_type: NamedType) -> Result<()> {
        let type_name = get_type_name(&new_type);

        // Check if a type with this name already exists
        if let Some(existing_type) = self.types.iter().find(|t| get_type_name(t) == type_name) {
            // If the structures are different, this is an error
            if !types_are_structurally_equal(existing_type, &new_type) {
                bail!(
                    "Type name collision: a type named '{}' already exists with a different structure",
                    type_name
                );
            }
            // Otherwise, it's a duplicate of the same type, so we can skip it
            return Ok(());
        }

        // New type, add it
        self.type_names.insert(type_name.to_string());
        self.types.push(new_type);
        Ok(())
    }

    fn extract_schema_name(&self, schema: &Schema) -> Result<String> {
        let obj = schema
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("Schema must be an object to extract name"))?;

        // Try to get the title field
        if let Some(title) = obj.get("title").and_then(|v| v.as_str()) {
            return Ok(title.to_string());
        }

        // Fallback: try to infer from $ref in the schema itself
        if let Some(ref_str) = obj.get("$ref").and_then(|v| v.as_str()) {
            if let Ok(name) = extract_ref_name(ref_str) {
                return Ok(name);
            }
        }

        // Last resort: generate a unique name based on existing types
        let mut counter = 1;
        loop {
            let candidate = format!("AnonymousType{}", counter);
            if !self.type_names.contains(&candidate) {
                return Ok(candidate);
            }
            counter += 1;
        }
    }

    /// Helper to extract schema name and add schema to types list
    /// Returns the schema name for use in references
    fn add_schema_and_get_name(&mut self, schema: &Schema) -> Result<String> {
        let name = self.extract_schema_name(schema)?;
        self.append_types_from_schema(schema, &name)?;
        Ok(name)
    }
}

fn get_type_name(named_type: &NamedType) -> &str {
    match named_type {
        NamedType::Object(obj) => &obj.name,
        NamedType::Union(union) => &union.name,
        NamedType::Enum(enum_type) => &enum_type.name,
    }
}

fn extract_ref_name(reference: &str) -> Result<String> {
    // References typically look like "#/definitions/TypeName" or "#/$defs/TypeName"
    if let Some(name) = reference.strip_prefix("#/definitions/") {
        Ok(name.to_string())
    } else if let Some(name) = reference.strip_prefix("#/$defs/") {
        Ok(name.to_string())
    } else {
        bail!("Unsupported reference format: {}", reference)
    }
}
