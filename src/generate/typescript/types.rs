use crate::aat::*;
use crate::generate::writer::CodeWriter;
use anyhow::Result;

use super::utils::*;

/// Check if a field type is a reference (either directly or wrapped in Optional)
fn is_reference_type(field_type: &FieldType) -> bool {
    matches!(field_type, FieldType::Reference(_)) ||
    matches!(field_type, FieldType::Optional(inner) if matches!(&**inner, FieldType::Reference(_)))
}

pub fn generate_type(writer: &mut CodeWriter, named_type: &NamedType) -> Result<()> {
    match named_type {
        NamedType::Object(obj) => generate_object_type(writer, obj),
        NamedType::Union(union) => generate_union_type(writer, union),
        NamedType::Enum(enum_type) => generate_enum_type(writer, enum_type),
    }
}

fn generate_object_type(writer: &mut CodeWriter, obj: &ObjectType) -> Result<()> {
    // Convert field types outside the closure to avoid borrowing issues
    let fields: Vec<_> = obj
        .fields
        .iter()
        .map(|f| {
            let is_optional = matches!(&*f.r#type, FieldType::Optional(_));
            let ts_type = if is_optional {
                // For optional fields, get the inner type without the | undefined
                if let FieldType::Optional(inner) = &*f.r#type {
                    field_type_to_ts(inner)
                } else {
                    field_type_to_ts(&f.r#type)
                }
            } else {
                field_type_to_ts(&f.r#type)
            };
            (
                to_camel_case(&f.name),
                ts_type,
                f.name.clone(),
                is_optional,
            )
        })
        .collect();

    // Check for duplicate field names after camelCase conversion
    let mut seen = std::collections::HashSet::new();
    for (camel_name, _, original_name, _) in &fields {
        if !seen.insert(camel_name.clone()) {
            anyhow::bail!(
                "Duplicate field name '{}' in type '{}' after camelCase conversion (original: '{}')",
                camel_name,
                obj.name,
                original_name
            );
        }
    }

    writer
        .block_with_newline(&format!("export interface {} {{", obj.name), "}", |w| {
            for (name, ts_type, _, is_optional) in fields {
                let quoted_name = quote_if_needed(&name);
                if is_optional {
                    w.line(&format!("{}?: {};", quoted_name, ts_type));
                } else {
                    w.line(&format!("{}: {};", quoted_name, ts_type));
                }
            }
        });
    Ok(())
}

fn generate_union_type(writer: &mut CodeWriter, union: &UnionType) -> Result<()> {
    writer.line(&format!("export type {} =", union.name));
    writer.indent();

    // Check for duplicate variant names in literal variants
    let mut seen_literals = std::collections::HashSet::new();

    for (i, variant) in union.variants.iter().enumerate() {
        let separator = if i == union.variants.len() - 1 {
            ";"
        } else {
            " |"
        };

        match &*variant.mode {
            UnionTypeVariantMode::Object(obj) => {
                // Inline object type - check for duplicate field names
                let mut parts = Vec::new();
                let mut seen_fields = std::collections::HashSet::new();
                for field in &obj.fields {
                    let is_optional = matches!(&*field.r#type, FieldType::Optional(_));
                    let field_type = if is_optional {
                        // For optional fields, get the inner type
                        if let FieldType::Optional(inner) = &*field.r#type {
                            field_type_to_ts(inner)
                        } else {
                            field_type_to_ts(&field.r#type)
                        }
                    } else {
                        field_type_to_ts(&field.r#type)
                    };
                    let field_name = to_camel_case(&field.name);
                    if !seen_fields.insert(field_name.clone()) {
                        anyhow::bail!(
                            "Duplicate field name '{}' in union type '{}' after camelCase conversion (original: '{}')",
                            field_name,
                            union.name,
                            field.name
                        );
                    }
                    let quoted_field_name = quote_if_needed(&field_name);
                    if is_optional {
                        parts.push(format!("{}?: {}", quoted_field_name, field_type));
                    } else {
                        parts.push(format!("{}: {}", quoted_field_name, field_type));
                    }
                }

                // Determine how to represent this variant
                if let Some(variant_name) = &variant.name {
                    let quoted_variant_name = quote_if_needed(variant_name);

                    // Check if this is a newtype pattern (single field that is a reference)
                    let is_newtype = obj.fields.len() == 1 && is_reference_type(&obj.fields[0].r#type);

                    // Check if this is a tuple variant (single field with same name as variant)
                    // This indicates a Rust tuple enum variant that should be flattened
                    let is_tuple_variant = obj.fields.len() == 1 &&
                                          obj.fields[0].name == *variant_name;

                    if is_newtype {
                        // Newtype pattern: wrap with variant name
                        let field = &obj.fields[0];
                        let field_type = field_type_to_ts(&field.r#type);
                        writer.line(&format!("{{ {}: {} }}{}", quoted_variant_name, field_type, separator));
                    } else if is_tuple_variant {
                        // Tuple variant: flatten completely (remove variant name wrapper)
                        writer.line(&format!("{{ {} }}{}", parts.join(", "), separator));
                    } else {
                        // Regular struct variant: wrap with variant name
                        let inner_obj = format!("{{ {} }}", parts.join(", "));
                        writer.line(&format!("{{ {}: {} }}{}", quoted_variant_name, inner_obj, separator));
                    }
                } else {
                    // No variant name: use fields directly
                    writer.line(&format!("{{ {} }}{}", parts.join(", "), separator));
                }
            }
            UnionTypeVariantMode::Literal(lit) => {
                let lit_str = literal_to_ts(lit);
                if let LiteralType::String(s) = lit {
                    // Still check for duplicates with original string values
                    if !seen_literals.insert(s.clone()) {
                        anyhow::bail!(
                            "Duplicate literal variant '{}' in union type '{}'",
                            s,
                            union.name
                        );
                    }
                }
                writer.line(&format!("{}{}", lit_str, separator));
            }
        }
    }

    writer.dedent();
    writer.empty_line();
    Ok(())
}

fn generate_enum_type(writer: &mut CodeWriter, enum_type: &EnumType) -> Result<()> {
    writer.line(&format!("export type {} =", enum_type.name));
    writer.indent();

    // Check for duplicate variants after camelCase conversion
    let mut seen = std::collections::HashSet::new();

    for (i, variant) in enum_type.variants.iter().enumerate() {
        let separator = if i == enum_type.variants.len() - 1 {
            ";"
        } else {
            " |"
        };

        let lit_str = literal_to_ts(&variant.value);
        if let LiteralType::String(s) = &variant.value {
            // Still check for duplicates with original string values
            if !seen.insert(s.clone()) {
                anyhow::bail!(
                    "Duplicate enum variant '{}' in enum type '{}'",
                    s,
                    enum_type.name
                );
            }
        }
        writer.line(&format!("{}{}", lit_str, separator));
    }

    writer.dedent();
    writer.empty_line();
    Ok(())
}
