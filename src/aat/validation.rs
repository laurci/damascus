use anyhow::{Result, bail};
use super::types::*;

/// Validates that all type references in the AAT resolve to actual types
pub fn validate_references(
    services: &[Service],
    types: &[NamedType],
) -> Result<()> {
    // Collect all type names for quick lookup
    let valid_type_names: std::collections::HashSet<_> =
        types.iter().map(|t| get_type_name(t)).collect();

    // Check all references in services/endpoints
    for service in services {
        for endpoint in &service.endpoints {
            // Check path parameter types
            for segment in &endpoint.path {
                if let PathSegment::Parameter { name, r#type } = segment {
                    validate_field_type_references(r#type, &valid_type_names)
                        .map_err(|e| anyhow::anyhow!(
                            "Invalid reference in path parameter '{}' of endpoint '{}': {}",
                            name, endpoint.name, e
                        ))?;
                }
            }

            // Check query type
            if let Some(query_type) = &endpoint.query {
                validate_field_type_references(query_type, &valid_type_names)
                    .map_err(|e| anyhow::anyhow!(
                        "Invalid reference in query of endpoint '{}': {}",
                        endpoint.name, e
                    ))?;
            }

            // Check body type
            if let Some(body_type) = &endpoint.body {
                validate_field_type_references(body_type, &valid_type_names)
                    .map_err(|e| anyhow::anyhow!(
                        "Invalid reference in body of endpoint '{}': {}",
                        endpoint.name, e
                    ))?;
            }

            // Check response type
            validate_field_type_references(&endpoint.response, &valid_type_names)
                .map_err(|e| anyhow::anyhow!(
                    "Invalid reference in response of endpoint '{}': {}",
                    endpoint.name, e
                ))?;
        }
    }

    // Check all references within types themselves
    for named_type in types {
        match named_type {
            NamedType::Object(obj) => {
                for field in &obj.fields {
                    validate_field_type_references(&field.r#type, &valid_type_names)
                        .map_err(|e| anyhow::anyhow!(
                            "Invalid reference in field '{}' of type '{}': {}",
                            field.name, obj.name, e
                        ))?;
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
        FieldType::Optional(inner) | FieldType::List(inner) | FieldType::Map(inner) => {
            validate_field_type_references(inner, valid_type_names)
        }
        FieldType::Intersection(types) => {
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
        Type::Stream(_) => bail!("Path parameter cannot be Stream type - streams are not supported in URL paths"),
        Type::List(_) => bail!("Path parameter cannot be List type - use query parameters for arrays"),
        Type::Optional(_) => bail!("Path parameter cannot be Optional type - path parameters are always required"),
        Type::Tuple(_) => bail!("Path parameter cannot be Tuple type - use a structured type instead"),
        Type::NamedTuple(_) => bail!("Path parameter cannot be NamedTuple type - use a structured type instead"),
    }
}
