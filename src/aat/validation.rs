use super::types::*;
use anyhow::{Result, bail};

/// Validates that all type references in the AAT resolve to actual types
pub fn validate_references(services: &[Service], types: &[NamedType]) -> Result<()> {
    // Collect all type names for quick lookup
    let valid_type_names: std::collections::HashSet<_> =
        types.iter().map(|t| get_type_name(t)).collect();

    // Check all references in services/endpoints
    for service in services {
        for endpoint in &service.endpoints {
            // Check path parameter types
            for segment in &endpoint.path {
                if let PathSegment::Parameter { name, r#type } = segment {
                    validate_field_type_references(r#type, &valid_type_names).map_err(|e| {
                        anyhow::anyhow!(
                            "Invalid reference in path parameter '{}' of endpoint '{}': {}",
                            name,
                            endpoint.name,
                            e
                        )
                    })?;

                    // Validate that path parameter is string-serializable
                    validate_path_parameter_is_stringifiable(r#type, &endpoint.name, name, types)?;
                }
            }

            // Check query type
            if let Some(query_type) = &endpoint.query {
                validate_field_type_references(query_type, &valid_type_names).map_err(|e| {
                    anyhow::anyhow!(
                        "Invalid reference in query of endpoint '{}': {}",
                        endpoint.name,
                        e
                    )
                })?;
            }

            // Check body type
            if let Some(body_type) = &endpoint.body {
                validate_field_type_references(body_type, &valid_type_names).map_err(|e| {
                    anyhow::anyhow!(
                        "Invalid reference in body of endpoint '{}': {}",
                        endpoint.name,
                        e
                    )
                })?;
            }

            // Check response type
            validate_field_type_references(&endpoint.response, &valid_type_names).map_err(|e| {
                anyhow::anyhow!(
                    "Invalid reference in response of endpoint '{}': {}",
                    endpoint.name,
                    e
                )
            })?;
        }
    }

    // Check all references within types themselves
    for named_type in types {
        match named_type {
            NamedType::Object(obj) => {
                for field in &obj.fields {
                    validate_field_type_references(&field.r#type, &valid_type_names).map_err(
                        |e| {
                            anyhow::anyhow!(
                                "Invalid reference in field '{}' of type '{}': {}",
                                field.name,
                                obj.name,
                                e
                            )
                        },
                    )?;
                }
            }
            NamedType::Union(union) => {
                for variant in &union.variants {
                    if let UnionTypeVariantMode::Object(obj) = &*variant.mode {
                        for field in &obj.fields {
                            validate_field_type_references(&field.r#type, &valid_type_names)
                                .map_err(|e| anyhow::anyhow!(
                                    "Invalid reference in field '{}' of union variant '{}' in type '{}': {}",
                                    field.name, variant.name.as_deref().unwrap_or("unnamed"), union.name, e
                                ))?;
                        }
                    }
                }
            }
            NamedType::Enum(_) => {
                // Enums don't have references
            }
        }
    }

    Ok(())
}

fn validate_field_type_references(
    field_type: &FieldType,
    valid_type_names: &std::collections::HashSet<&str>,
) -> Result<()> {
    match field_type {
        FieldType::Reference(ref_name) => {
            if !valid_type_names.contains(ref_name.as_str()) {
                bail!("Reference to undefined type '{}'", ref_name);
            }
            Ok(())
        }
        FieldType::Optional(inner) | FieldType::List(inner) | FieldType::Map(inner) | FieldType::Stream(inner) => {
            validate_field_type_references(inner, valid_type_names)
        }
        FieldType::Intersection(types) => {
            for t in types {
                validate_field_type_references(t, valid_type_names)?;
            }
            Ok(())
        }
        FieldType::Tuple(types) => {
            for t in types {
                validate_field_type_references(t, valid_type_names)?;
            }
            Ok(())
        }
        FieldType::Primitive(_) | FieldType::Literal(_) | FieldType::Any => {
            // These don't contain references
            Ok(())
        }
    }
}

fn get_type_name(named_type: &NamedType) -> &str {
    match named_type {
        NamedType::Object(obj) => &obj.name,
        NamedType::Union(union) => &union.name,
        NamedType::Enum(enum_type) => &enum_type.name,
    }
}

pub fn validate_path_parameter_type(r#type: &crate::spec::Type) -> Result<()> {
    use crate::spec::Type;

    match r#type {
        Type::Schema(_) => Ok(()),
        Type::Void => bail!("Path parameter cannot be Void type"),
        Type::Stream(_) => {
            bail!("Path parameter cannot be Stream type - streams are not supported in URL paths")
        }
        Type::List(_) => {
            bail!("Path parameter cannot be List type - use query parameters for arrays")
        }
        Type::Optional(_) => {
            bail!("Path parameter cannot be Optional type - path parameters are always required")
        }
        Type::Tuple(_) => {
            bail!("Path parameter cannot be Tuple type - use a structured type instead")
        }
        Type::NamedTuple(_) => {
            bail!("Path parameter cannot be NamedTuple type - use a structured type instead")
        }
    }
}

/// Validates that a path parameter type can be serialized to a string
fn validate_path_parameter_is_stringifiable(
    field_type: &FieldType,
    endpoint_name: &str,
    param_name: &str,
    types: &[NamedType],
) -> Result<()> {
    match field_type {
        FieldType::Primitive(PrimitiveType::String(_)) => Ok(()),
        FieldType::Primitive(PrimitiveType::Int) => Ok(()),
        FieldType::Primitive(PrimitiveType::Float) => Ok(()),
        FieldType::Primitive(PrimitiveType::Bool) => Ok(()),
        FieldType::Literal(_) => Ok(()),
        FieldType::Reference(type_name) => {
            // Look up the referenced type
            let named_type = types
                .iter()
                .find(|t| get_type_name(t) == type_name)
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "Path parameter '{}' in endpoint '{}' references undefined type '{}'",
                        param_name,
                        endpoint_name,
                        type_name
                    )
                })?;

            match named_type {
                NamedType::Enum(enum_type) => {
                    // Validate all enum variants are string literals
                    for variant in &enum_type.variants {
                        if !matches!(variant.value, LiteralType::String(_)) {
                            bail!(
                                "Path parameter '{}' in endpoint '{}' references enum '{}' which has non-string variant. Only string enums are allowed in path parameters.",
                                param_name,
                                endpoint_name,
                                type_name
                            );
                        }
                    }
                    Ok(())
                }
                NamedType::Object(_) => {
                    bail!(
                        "Path parameter '{}' in endpoint '{}' cannot be an object type '{}'. Path parameters must be primitives or string enums.",
                        param_name,
                        endpoint_name,
                        type_name
                    )
                }
                NamedType::Union(_) => {
                    bail!(
                        "Path parameter '{}' in endpoint '{}' cannot be a union type '{}'. Path parameters must be primitives or string enums.",
                        param_name,
                        endpoint_name,
                        type_name
                    )
                }
            }
        }
        FieldType::Optional(_) => {
            bail!(
                "Path parameter '{}' in endpoint '{}' cannot be optional. Path parameters are always required.",
                param_name,
                endpoint_name
            )
        }
        FieldType::List(_) => {
            bail!(
                "Path parameter '{}' in endpoint '{}' cannot be a list. Use query parameters for arrays.",
                param_name,
                endpoint_name
            )
        }
        FieldType::Map(_) => {
            bail!(
                "Path parameter '{}' in endpoint '{}' cannot be a map. Path parameters must be primitives or string enums.",
                param_name,
                endpoint_name
            )
        }
        FieldType::Intersection(_) => {
            bail!(
                "Path parameter '{}' in endpoint '{}' cannot be an intersection type. Path parameters must be primitives or string enums.",
                param_name,
                endpoint_name
            )
        }
        FieldType::Tuple(_) => {
            bail!(
                "Path parameter '{}' in endpoint '{}' cannot be a tuple type. Path parameters must be primitives or string enums.",
                param_name,
                endpoint_name
            )
        }
        FieldType::Stream(_) => {
            bail!(
                "Path parameter '{}' in endpoint '{}' cannot be a stream type. Path parameters must be primitives or string enums.",
                param_name,
                endpoint_name
            )
        }
        FieldType::Any => {
            bail!(
                "Path parameter '{}' in endpoint '{}' cannot be 'any' type. Path parameters must be primitives or string enums.",
                param_name,
                endpoint_name
            )
        }
    }
}
