use crate::aat::*;
use crate::generate::writer::CodeWriter;
use anyhow::Result;

use super::utils::*;

pub fn generate_serializer(writer: &mut CodeWriter, named_type: &NamedType) -> Result<()> {
    match named_type {
        NamedType::Object(obj) => generate_object_serializer(writer, obj),
        NamedType::Union(union) => generate_union_serializer(writer, union),
        NamedType::Enum(enum_type) => generate_enum_serializer(writer, enum_type),
    }
}

pub fn generate_deserializer(writer: &mut CodeWriter, named_type: &NamedType) -> Result<()> {
    match named_type {
        NamedType::Object(obj) => generate_object_deserializer(writer, obj),
        NamedType::Union(union) => generate_union_deserializer(writer, union),
        NamedType::Enum(enum_type) => generate_enum_deserializer(writer, enum_type),
    }
}

fn generate_object_serializer(writer: &mut CodeWriter, obj: &ObjectType) -> Result<()> {
    let func_name = format!("serialize{}", obj.name);
    writer.block(
        &format!("function {}(value: {}): any {{", func_name, obj.name),
        "}",
        |w| {
            w.line("return {");
            w.indent();
            for field in &obj.fields {
                let camel_name = to_camel_case(&field.name);
                let original_name = &field.name;

                // Check if field needs serialization
                if needs_serialization(&field.r#type) {
                    let serializer_expr = get_serializer_expr(&field.r#type, &format!("value.{}", camel_name));
                    w.line(&format!(
                        "\"{}\": {},",
                        original_name, serializer_expr
                    ));
                } else {
                    w.line(&format!("\"{}\": value.{},", original_name, camel_name));
                }
            }
            w.dedent();
            w.line("};");
        },
    );
    Ok(())
}

fn generate_object_deserializer(writer: &mut CodeWriter, obj: &ObjectType) -> Result<()> {
    let func_name = format!("deserialize{}", obj.name);
    writer.block(
        &format!("function {}(value: any): {} {{", func_name, obj.name),
        "}",
        |w| {
            w.line("return {");
            w.indent();
            for field in &obj.fields {
                let camel_name = to_camel_case(&field.name);
                let original_name = &field.name;

                // Check if field needs deserialization
                if needs_serialization(&field.r#type) {
                    let deserializer_expr = get_deserializer_expr(&field.r#type, &format!("value[\"{}\"]", original_name));
                    w.line(&format!(
                        "{}: {},",
                        camel_name, deserializer_expr
                    ));
                } else {
                    w.line(&format!("{}: value[\"{}\"],", camel_name, original_name));
                }
            }
            w.dedent();
            w.line("};");
        },
    );
    Ok(())
}

fn generate_union_serializer(writer: &mut CodeWriter, union: &UnionType) -> Result<()> {
    // Check if any variant has object fields that need serialization
    let has_objects = union.variants.iter().any(|v| {
        matches!(&*v.mode, UnionTypeVariantMode::Object(obj) if !obj.fields.is_empty())
    });
    let has_literals = union.variants.iter().any(|v| {
        matches!(&*v.mode, UnionTypeVariantMode::Literal(_))
    });

    if has_objects {
        let func_name = format!("serialize{}", union.name);
        writer.block(
            &format!("function {}(value: {}): any {{", func_name, union.name),
            "}",
            |w| {
                if has_literals {
                    w.line("// Union serialization: pass through literal values, convert object field names");
                    w.line("if (typeof value === 'string' || typeof value === 'number' || typeof value === 'boolean') {");
                    w.indent();
                    w.line("return value;");
                    w.dedent();
                    w.line("}");
                }
                w.line("// Handle object variants");
                w.line("const result: any = {};");
                w.line("for (const [key, val] of Object.entries(value)) {");
                w.indent();
                w.line("const snakeKey = key.replace(/[A-Z]/g, (m: string) => '_' + m.toLowerCase());");
                w.line("result[snakeKey] = val;");
                w.dedent();
                w.line("}");
                w.line("return result;");
            },
        );
    }
    Ok(())
}

fn generate_union_deserializer(writer: &mut CodeWriter, union: &UnionType) -> Result<()> {
    // Check if any variant has object fields that need deserialization
    let has_objects = union.variants.iter().any(|v| {
        matches!(&*v.mode, UnionTypeVariantMode::Object(obj) if !obj.fields.is_empty())
    });
    let has_literals = union.variants.iter().any(|v| {
        matches!(&*v.mode, UnionTypeVariantMode::Literal(_))
    });

    if has_objects {
        let func_name = format!("deserialize{}", union.name);
        let union_name = &union.name;
        writer.block(
            &format!("function {}(value: any): {} {{", func_name, union_name),
            "}",
            |w| {
                if has_literals {
                    w.line("// Union deserialization: pass through literal values, convert object field names");
                    w.line("if (typeof value === 'string' || typeof value === 'number' || typeof value === 'boolean') {");
                    w.indent();
                    w.line(&format!("return value as {};", union_name));
                    w.dedent();
                    w.line("}");
                }
                w.line("// Handle object variants");
                w.line("const result: any = {};");
                w.line("for (const [key, val] of Object.entries(value)) {");
                w.indent();
                w.line("const camelKey = key.replace(/_([a-z])/g, (_, m: string) => m.toUpperCase());");
                w.line("result[camelKey] = val;");
                w.dedent();
                w.line("}");
                w.line(&format!("return result as {};", union_name));
            },
        );
    }
    Ok(())
}

fn generate_enum_serializer(writer: &mut CodeWriter, enum_type: &EnumType) -> Result<()> {
    // Enums are just string literals, but we still generate pass-through functions
    // in case they're referenced by other types
    let func_name = format!("serialize{}", enum_type.name);
    writer.block(
        &format!("function {}(value: {}): any {{", func_name, enum_type.name),
        "}",
        |w| {
            w.line("return value;");
        },
    );
    Ok(())
}

fn generate_enum_deserializer(writer: &mut CodeWriter, enum_type: &EnumType) -> Result<()> {
    // Enums are just string literals, but we still generate pass-through functions
    // in case they're referenced by other types
    let func_name = format!("deserialize{}", enum_type.name);
    writer.block(
        &format!("function {}(value: any): {} {{", func_name, enum_type.name),
        "}",
        |w| {
            w.line("return value;");
        },
    );
    Ok(())
}

fn needs_serialization(field_type: &FieldType) -> bool {
    match field_type {
        FieldType::Reference(_) => true,
        FieldType::List(inner) => needs_serialization(inner),
        FieldType::Optional(inner) => needs_serialization(inner),
        _ => false,
    }
}

fn get_serializer_call(field_type: &FieldType) -> String {
    match field_type {
        FieldType::Reference(name) => format!("serialize{}", name),
        FieldType::List(inner) => {
            if needs_serialization(inner) {
                let inner_serializer = get_serializer_call(inner);
                format!("(v: any) => v.map((x: any) => {}(x))", inner_serializer)
            } else {
                "(v: any) => v".to_string()
            }
        }
        FieldType::Optional(inner) => {
            if needs_serialization(inner) {
                let inner_serializer = get_serializer_call(inner);
                format!(
                    "(v: any) => v !== undefined ? {}(v) : undefined",
                    inner_serializer
                )
            } else {
                "(v: any) => v".to_string()
            }
        }
        _ => "(v: any) => v".to_string(),
    }
}

fn get_deserializer_call(field_type: &FieldType) -> String {
    match field_type {
        FieldType::Reference(name) => format!("deserialize{}", name),
        FieldType::List(inner) => {
            if needs_serialization(inner) {
                let inner_deserializer = get_deserializer_call(inner);
                format!("(v: any) => v.map((x: any) => {}(x))", inner_deserializer)
            } else {
                "(v: any) => v".to_string()
            }
        }
        FieldType::Optional(inner) => {
            if needs_serialization(inner) {
                let inner_deserializer = get_deserializer_call(inner);
                format!(
                    "(v: any) => v !== undefined && v !== null ? {}(v) : undefined",
                    inner_deserializer
                )
            } else {
                "(v: any) => v".to_string()
            }
        }
        _ => "(v: any) => v".to_string(),
    }
}

/// Generate an inline serialization expression for a given value
fn get_serializer_expr(field_type: &FieldType, value_expr: &str) -> String {
    match field_type {
        FieldType::Reference(name) => format!("serialize{}({})", name, value_expr),
        FieldType::List(inner) => {
            if needs_serialization(inner) {
                let item_expr = get_serializer_expr(inner, "x");
                format!("{}.map((x: any) => {})", value_expr, item_expr)
            } else {
                value_expr.to_string()
            }
        }
        FieldType::Optional(inner) => {
            if needs_serialization(inner) {
                let inner_expr = get_serializer_expr(inner, value_expr);
                format!("{} !== undefined ? {} : undefined", value_expr, inner_expr)
            } else {
                value_expr.to_string()
            }
        }
        _ => value_expr.to_string(),
    }
}

/// Generate an inline deserialization expression for a given value
fn get_deserializer_expr(field_type: &FieldType, value_expr: &str) -> String {
    match field_type {
        FieldType::Reference(name) => format!("deserialize{}({})", name, value_expr),
        FieldType::List(inner) => {
            if needs_serialization(inner) {
                let item_expr = get_deserializer_expr(inner, "x");
                format!("{}.map((x: any) => {})", value_expr, item_expr)
            } else {
                value_expr.to_string()
            }
        }
        FieldType::Optional(inner) => {
            if needs_serialization(inner) {
                let inner_expr = get_deserializer_expr(inner, value_expr);
                format!("{} !== undefined && {} !== null ? {} : undefined", value_expr, value_expr, inner_expr)
            } else {
                value_expr.to_string()
            }
        }
        _ => value_expr.to_string(),
    }
}
