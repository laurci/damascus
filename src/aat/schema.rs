use anyhow::{Result, bail};
use schemars::Schema;
use serde_json::{Map, Value};
use super::types::*;

pub fn schema_to_type(schema: &Schema, name: &str) -> Result<NamedType> {
    // Check if it's a bool schema
    if let Some(b) = schema.as_bool() {
        if b {
            bail!("Schema bool(true) (any type) cannot be converted to a named type");
        } else {
            bail!("Schema bool(false) (no type) cannot be converted to a named type");
        }
    }

    // Must be an object schema
    let obj = schema
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("Schema must be an object or bool"))?;

    // Check if this is an enum (has enum property)
    if let Some(Value::Array(enum_values)) = obj.get("enum") {
        return schema_to_enum_type(name, enum_values, obj);
    }

    // Check if this is a union (has oneOf)
    if let Some(Value::Array(one_of)) = obj.get("oneOf") {
        return schema_to_union_type(name, one_of, obj);
    }

    // Otherwise treat as an object
    schema_to_object_type(name, obj)
}

fn schema_to_enum_type(
    name: &str,
    enum_values: &[Value],
    _schema_obj: &Map<String, Value>,
) -> Result<NamedType> {
    let variants: Result<Vec<EnumVariant>> = enum_values
        .iter()
        .map(|value| {
            let literal = json_value_to_literal(value)?;
            Ok(EnumVariant {
                value: literal,
                description: None,
            })
        })
        .collect();

    Ok(NamedType::Enum(EnumType {
        name: name.to_string(),
        variants: variants?,
    }))
}

fn schema_to_union_type(
    name: &str,
    one_of: &[Value],
    schema_obj: &Map<String, Value>,
) -> Result<NamedType> {
    // Check for discriminator
    let discriminator = if let Some(Value::Object(disc_obj)) = schema_obj.get("discriminator") {
        if let Some(Value::String(prop_name)) = disc_obj.get("propertyName") {
            let mapping = disc_obj
                .get("mapping")
                .and_then(|v| v.as_object())
                .map(|map_obj| {
                    map_obj
                        .iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect()
                });
            Some(Discriminator {
                property_name: prop_name.clone(),
                mapping,
            })
        } else {
            None
        }
    } else {
        None
    };

    let variants: Result<Vec<UnionTypeVariant>> = one_of
        .iter()
        .enumerate()
        .map(|(idx, variant_value)| {
            // Convert to Schema
            let variant_schema = Schema::try_from(variant_value.clone())
                .map_err(|_| anyhow::anyhow!("Invalid schema in oneOf"))?;

            if let Some(variant_obj) = variant_schema.as_object() {
                // Try to get a name from the title, or if it's an object with a single
                // required property, use that property name
                let variant_name =
                    if let Some(title) = variant_obj.get("title").and_then(|v| v.as_str()) {
                        title.to_string()
                    } else if variant_obj.get("type").and_then(|v| v.as_str()) == Some("object") {
                        // Check if this is an object with a single required property
                        if let Some(Value::Array(required)) = variant_obj.get("required") {
                            if required.len() == 1 {
                                if let Some(prop_name) = required[0].as_str() {
                                    prop_name.to_string()
                                } else {
                                    format!("Variant{}", idx)
                                }
                            } else {
                                format!("Variant{}", idx)
                            }
                        } else {
                            format!("Variant{}", idx)
                        }
                    } else {
                        format!("Variant{}", idx)
                    };

                // Check if it's a literal (enum with single value) or an object
                if let Some(Value::Array(enum_vals)) = variant_obj.get("enum") {
                    if enum_vals.len() == 1 {
                        let literal = json_value_to_literal(&enum_vals[0])?;
                        return Ok(UnionTypeVariant {
                            name: Some(variant_name),
                            mode: Box::new(UnionTypeVariantMode::Literal(literal)),
                        });
                    }
                }

                // It's an object type
                let object_type = schema_to_object_type(&variant_name, variant_obj)?;
                match object_type {
                    NamedType::Object(obj) => Ok(UnionTypeVariant {
                        name: Some(variant_name),
                        mode: Box::new(UnionTypeVariantMode::Object(obj)),
                    }),
                    _ => bail!("Expected object type in union variant"),
                }
            } else if variant_schema.as_bool().is_some() {
                bail!("Boolean schemas not supported in unions")
            } else {
                bail!("Invalid schema in oneOf")
            }
        })
        .collect();

    Ok(NamedType::Union(UnionType {
        name: name.to_string(),
        discriminator,
        variants: variants?,
    }))
}

fn schema_to_object_type(name: &str, schema_obj: &Map<String, Value>) -> Result<NamedType> {
    let mut fields = Vec::new();

    if let Some(Value::Object(properties)) = schema_obj.get("properties") {
        let required_set: std::collections::HashSet<_> = schema_obj
            .get("required")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
            .unwrap_or_default();

        for (field_name, field_value) in properties {
            let is_required = required_set.contains(field_name.as_str());

            // Convert to Schema
            let field_schema = Schema::try_from(field_value.clone())
                .map_err(|_| anyhow::anyhow!("Invalid field schema"))?;

            let field_type = schema_to_field_type(&field_schema)?;

            // Wrap in Optional if not required
            let field_type = if is_required {
                field_type
            } else {
                FieldType::Optional(Box::new(field_type))
            };

            let constraints = extract_constraints_from_object(field_value.as_object())?;

            fields.push(Field {
                name: field_name.clone(),
                r#type: Box::new(field_type),
                constraints,
            });
        }
    }

    Ok(NamedType::Object(ObjectType {
        name: name.to_string(),
        fields,
    }))
}

pub fn schema_to_field_type(schema: &Schema) -> Result<FieldType> {
    if let Some(b) = schema.as_bool() {
        return if b {
            Ok(FieldType::Any)
        } else {
            bail!("Schema bool(false) cannot be converted to a field type")
        };
    }

    let obj = schema
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("Schema must be an object or bool"))?;
    schema_object_to_field_type(obj)
}

fn schema_object_to_field_type(obj: &Map<String, Value>) -> Result<FieldType> {
    // Handle references
    if let Some(Value::String(reference)) = obj.get("$ref") {
        let name = extract_ref_name(reference)?;
        return Ok(FieldType::Reference(name));
    }

    // Handle allOf (intersection)
    if let Some(Value::Array(all_of)) = obj.get("allOf") {
        let types: Result<Vec<_>> = all_of
            .iter()
            .map(|v| {
                let schema = Schema::try_from(v.clone())
                    .map_err(|_| anyhow::anyhow!("Invalid schema in allOf"))?;
                schema_to_field_type(&schema)
            })
            .collect();
        return Ok(FieldType::Intersection(types?));
    }

    // Handle nullable (supports both OpenAPI 2.0 and modern JSON Schema)
    // OpenAPI 2.0: "nullable": true
    // Modern JSON Schema: "type": ["string", "null"]
    let is_nullable_v2 = obj
        .get("nullable")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Get the instance type(s)
    let instance_types = obj.get("type").and_then(|v| {
        if let Some(s) = v.as_str() {
            Some(vec![s])
        } else if let Some(arr) = v.as_array() {
            Some(arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
        } else {
            None
        }
    });

    // Check if "null" is in the type array (modern JSON Schema)
    let is_nullable_v3 = instance_types
        .as_ref()
        .map(|types| types.contains(&"null"))
        .unwrap_or(false);

    let is_nullable = is_nullable_v2 || is_nullable_v3;

    // Get the actual type (excluding "null")
    let type_str = instance_types
        .as_ref()
        .and_then(|types| types.iter().find(|&&t| t != "null").copied());

    // Special case: if the type is purely null (["null"] or "null")
    if instance_types.as_ref().map(|t| t.len()).unwrap_or(0) == 1 && is_nullable_v3 {
        return Ok(FieldType::Literal(LiteralType::Null));
    }

    let base_type = match type_str {
        Some("null") => FieldType::Literal(LiteralType::Null),
        Some("boolean") => FieldType::Primitive(PrimitiveType::Bool),
        Some("integer") => FieldType::Primitive(PrimitiveType::Int),
        Some("number") => FieldType::Primitive(PrimitiveType::Float),
        Some("string") => {
            let format = obj
                .get("format")
                .and_then(|v| v.as_str())
                .and_then(string_format_from_str);
            FieldType::Primitive(PrimitiveType::String(format))
        }
        Some("array") => {
            if let Some(items_value) = obj.get("items") {
                let items_schema = Schema::try_from(items_value.clone())
                    .map_err(|_| anyhow::anyhow!("Invalid items schema"))?;
                let item_type = schema_to_field_type(&items_schema)?;
                FieldType::List(Box::new(item_type))
            } else {
                FieldType::List(Box::new(FieldType::Any))
            }
        }
        Some("object") => {
            if let Some(additional_props_value) = obj.get("additionalProperties") {
                // This is a map/dictionary type
                let additional_props_schema = Schema::try_from(additional_props_value.clone())
                    .map_err(|_| anyhow::anyhow!("Invalid additionalProperties schema"))?;
                let value_type = if let Some(true) = additional_props_schema.as_bool() {
                    FieldType::Any
                } else if let Some(false) = additional_props_schema.as_bool() {
                    bail!("Map with no additional properties not supported")
                } else {
                    schema_to_field_type(&additional_props_schema)?
                };
                return Ok(FieldType::Map(Box::new(value_type)));
            }
            // For a plain object without properties, treat as Any
            FieldType::Any
        }
        None => {
            // No instance type specified - could be a reference or any
            FieldType::Any
        }
        Some(other) => bail!("Unsupported type: {}", other),
    };

    // Wrap in Optional if nullable
    if is_nullable {
        Ok(FieldType::Optional(Box::new(base_type)))
    } else {
        Ok(base_type)
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

fn string_format_from_str(format: &str) -> Option<StringFormat> {
    match format {
        "date-time" => Some(StringFormat::DateTime),
        "date" => Some(StringFormat::Date),
        "time" => Some(StringFormat::Time),
        "uuid" => Some(StringFormat::Uuid),
        "email" => Some(StringFormat::Email),
        "uri" => Some(StringFormat::Uri),
        "hostname" => Some(StringFormat::Hostname),
        "ipv4" => Some(StringFormat::Ipv4),
        "ipv6" => Some(StringFormat::Ipv6),
        _ => None,
    }
}

fn json_value_to_literal(value: &serde_json::Value) -> Result<LiteralType> {
    match value {
        serde_json::Value::String(s) => Ok(LiteralType::String(s.clone())),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(LiteralType::Int(i))
            } else if let Some(f) = n.as_f64() {
                Ok(LiteralType::Float(f))
            } else {
                bail!("Unsupported number type in enum")
            }
        }
        serde_json::Value::Bool(b) => Ok(LiteralType::Bool(*b)),
        serde_json::Value::Null => Ok(LiteralType::Null),
        _ => bail!("Unsupported JSON value type in enum"),
    }
}

fn extract_constraints_from_object(obj: Option<&Map<String, Value>>) -> Result<Option<Constraints>> {
    let obj = match obj {
        Some(o) => o,
        None => return Ok(None),
    };

    let mut constraints = Constraints {
        minimum: None,
        maximum: None,
        exclusive_minimum: None,
        exclusive_maximum: None,
        multiple_of: None,
        min_length: None,
        max_length: None,
        pattern: None,
        min_items: None,
        max_items: None,
        unique_items: None,
    };

    let mut has_any = false;

    // Extract number constraints
    // Handle both JSON Schema formats:
    // - Draft 4: minimum + exclusiveMinimum (boolean)
    // - Draft 6+: minimum OR exclusiveMinimum (number)

    // Check for Draft 6+ style first (exclusiveMinimum as number)
    if let Some(exclusive_min) = obj.get("exclusiveMinimum").and_then(|v| v.as_f64()) {
        constraints.exclusive_minimum = Some(exclusive_min);
        has_any = true;
    } else if let Some(min) = obj.get("minimum").and_then(|v| v.as_f64()) {
        // Check if there's a Draft 4 style boolean exclusiveMinimum
        let is_exclusive = obj.get("exclusiveMinimum").and_then(|v| v.as_bool()).unwrap_or(false);
        if is_exclusive {
            constraints.exclusive_minimum = Some(min);
        } else {
            constraints.minimum = Some(min);
        }
        has_any = true;
    }

    // Same for maximum
    if let Some(exclusive_max) = obj.get("exclusiveMaximum").and_then(|v| v.as_f64()) {
        constraints.exclusive_maximum = Some(exclusive_max);
        has_any = true;
    } else if let Some(max) = obj.get("maximum").and_then(|v| v.as_f64()) {
        let is_exclusive = obj.get("exclusiveMaximum").and_then(|v| v.as_bool()).unwrap_or(false);
        if is_exclusive {
            constraints.exclusive_maximum = Some(max);
        } else {
            constraints.maximum = Some(max);
        }
        has_any = true;
    }

    if let Some(multiple) = obj.get("multipleOf").and_then(|v| v.as_f64()) {
        constraints.multiple_of = Some(multiple);
        has_any = true;
    }

    // Extract string constraints
    if let Some(min_len) = obj.get("minLength").and_then(|v| v.as_u64()) {
        constraints.min_length = Some(min_len as usize);
        has_any = true;
    }
    if let Some(max_len) = obj.get("maxLength").and_then(|v| v.as_u64()) {
        constraints.max_length = Some(max_len as usize);
        has_any = true;
    }
    if let Some(pattern) = obj.get("pattern").and_then(|v| v.as_str()) {
        constraints.pattern = Some(pattern.to_string());
        has_any = true;
    }

    // Extract array constraints
    if let Some(min_items) = obj.get("minItems").and_then(|v| v.as_u64()) {
        constraints.min_items = Some(min_items as usize);
        has_any = true;
    }
    if let Some(max_items) = obj.get("maxItems").and_then(|v| v.as_u64()) {
        constraints.max_items = Some(max_items as usize);
        has_any = true;
    }
    if let Some(unique) = obj.get("uniqueItems").and_then(|v| v.as_bool()) {
        constraints.unique_items = Some(unique);
        has_any = true;
    }

    if has_any {
        // Validate constraints before returning
        constraints.validate()?;
        Ok(Some(constraints))
    } else {
        Ok(None)
    }
}
