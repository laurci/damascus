use crate::aat::*;
use crate::generate::writer::CodeWriter;
use anyhow::Result;

use super::utils::*;

/// Check if a field type is a reference (either directly or wrapped in Optional)
fn is_reference_type(field_type: &FieldType) -> bool {
    matches!(field_type, FieldType::Reference(_)) ||
    matches!(field_type, FieldType::Optional(inner) if matches!(&**inner, FieldType::Reference(_)))
}

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
        &format!("export function {}(value: {}): any {{", func_name, obj.name),
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
        &format!("export function {}(value: any): {} {{", func_name, obj.name),
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
            &format!("export function {}(value: {}): any {{", func_name, union.name),
            "}",
            |w| {
                if has_literals {
                    w.line("// Union serialization: pass through literal values");
                    w.line("if (typeof value === 'string' || typeof value === 'number' || typeof value === 'boolean') {");
                    w.indent();
                    w.line("return value;");
                    w.dedent();
                    w.line("}");
                }

                // Handle object variants
                w.line("// Handle object variants");
                w.line("const result: any = {};");
                w.line("for (const [key, val] of Object.entries(value)) {");
                w.indent();

                // Generate handlers for named variants
                for variant in &union.variants {
                    if let UnionTypeVariantMode::Object(obj) = &*variant.mode {
                        if let Some(variant_name) = &variant.name {
                            // Check if this is a newtype pattern
                            let is_newtype = obj.fields.len() == 1 && is_reference_type(&obj.fields[0].r#type);

                            // Check if this is a tuple variant (single field with same name as variant)
                            let is_tuple_variant = obj.fields.len() == 1 &&
                                                  obj.fields[0].name == *variant_name;

                            if is_tuple_variant {
                                // Tuple variant: serialize fields directly without variant wrapper
                                let field = &obj.fields[0];
                                let camel_key = to_camel_case(&field.name);
                                let original_key = &field.name;

                                w.line(&format!("if (key === \"{}\") {{", camel_key));
                                w.indent();
                                if needs_serialization(&field.r#type) {
                                    let serializer_expr = get_serializer_expr(&field.r#type, "val as any");
                                    w.line(&format!("result[\"{}\"] = {};", original_key, serializer_expr));
                                } else {
                                    w.line(&format!("result[\"{}\"] = val;", original_key));
                                }
                                w.line("continue;");
                                w.dedent();
                                w.line("}");
                            } else {
                                // Regular variant: look for variant name key
                                w.line(&format!("if (key === \"{}\") {{", variant_name));
                                w.indent();

                                if is_newtype {
                                    // Newtype pattern: serialize the value directly
                                    let field = &obj.fields[0];
                                    if needs_serialization(&field.r#type) {
                                        let serializer_expr = get_serializer_expr(&field.r#type, "val as any");
                                        w.line(&format!("result[\"{}\"] = {};", variant_name, serializer_expr));
                                    } else {
                                        w.line(&format!("result[\"{}\"] = val;", variant_name));
                                    }
                                } else {
                                    // Regular struct variant: serialize the nested object
                                    w.line("const inner: any = {};");
                                    w.line("for (const [innerKey, innerVal] of Object.entries(val as any)) {");
                                    w.indent();

                                    // Check if any fields need serialization
                                    let has_serializable_fields = obj.fields.iter().any(|f| needs_serialization(&f.r#type));

                                    if has_serializable_fields {
                                        for field in &obj.fields {
                                            let camel_name = to_camel_case(&field.name);
                                            let original_name = &field.name;

                                            if needs_serialization(&field.r#type) {
                                                w.line(&format!("if (innerKey === \"{}\") {{", camel_name));
                                                w.indent();
                                                let serializer_expr = get_serializer_expr(&field.r#type, "innerVal as any");
                                                w.line(&format!("inner[\"{}\"] = {};", original_name, serializer_expr));
                                                w.line("continue;");
                                                w.dedent();
                                                w.line("}");
                                            }
                                        }
                                    }

                                    w.line("const snakeKey = innerKey.replace(/[A-Z]/g, (m: string) => '_' + m.toLowerCase());");
                                    w.line("inner[snakeKey] = innerVal;");
                                    w.dedent();
                                    w.line("}");
                                    w.line(&format!("result[\"{}\"] = inner;", variant_name));
                                }

                                w.line("continue;");
                                w.dedent();
                                w.line("}");
                            }
                        } else {
                            // Unnamed variant: handle field serialization directly
                            if obj.fields.len() == 1 {
                                let field = &obj.fields[0];
                                let camel_key = to_camel_case(&field.name);
                                let original_key = &field.name;

                                if needs_serialization(&field.r#type) {
                                    w.line(&format!("if (key === \"{}\") {{", camel_key));
                                    w.indent();
                                    let serializer_expr = get_serializer_expr(&field.r#type, "val as any");
                                    w.line(&format!("result[\"{}\"] = {};", original_key, serializer_expr));
                                    w.line("continue;");
                                    w.dedent();
                                    w.line("}");
                                }
                            }
                        }
                    }
                }

                // Fallback: simple key transformation
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
            &format!("export function {}(value: any): {} {{", func_name, union_name),
            "}",
            |w| {
                if has_literals {
                    w.line("// Union deserialization: pass through literal values");
                    w.line("if (typeof value === 'string' || typeof value === 'number' || typeof value === 'boolean') {");
                    w.indent();
                    w.line(&format!("return value as {};", union_name));
                    w.dedent();
                    w.line("}");
                }

                // Handle object variants
                w.line("// Handle object variants");
                w.line("const result: any = {};");
                w.line("for (const [key, val] of Object.entries(value)) {");
                w.indent();

                // Generate handlers for named variants
                for variant in &union.variants {
                    if let UnionTypeVariantMode::Object(obj) = &*variant.mode {
                        if let Some(variant_name) = &variant.name {
                            // Check if this is a newtype pattern
                            let is_newtype = obj.fields.len() == 1 && is_reference_type(&obj.fields[0].r#type);

                            // Check if this is a tuple variant (single field with same name as variant)
                            let is_tuple_variant = obj.fields.len() == 1 &&
                                                  obj.fields[0].name == *variant_name;

                            if is_tuple_variant {
                                // Tuple variant: deserialize fields directly without variant wrapper
                                let field = &obj.fields[0];
                                let camel_key = to_camel_case(&field.name);
                                let original_key = &field.name;

                                w.line(&format!("if (key === \"{}\") {{", original_key));
                                w.indent();
                                if needs_serialization(&field.r#type) {
                                    let deserializer_expr = get_deserializer_expr(&field.r#type, "val as any");
                                    w.line(&format!("result[\"{}\"] = {};", camel_key, deserializer_expr));
                                } else {
                                    w.line(&format!("result[\"{}\"] = val;", camel_key));
                                }
                                w.line("continue;");
                                w.dedent();
                                w.line("}");
                            } else {
                                // Regular variant: look for variant name key
                                w.line(&format!("if (key === \"{}\") {{", variant_name));
                                w.indent();

                                if is_newtype {
                                    // Newtype pattern: deserialize the value directly
                                    let field = &obj.fields[0];
                                    if needs_serialization(&field.r#type) {
                                        let deserializer_expr = get_deserializer_expr(&field.r#type, "val as any");
                                        w.line(&format!("result[\"{}\"] = {};", variant_name, deserializer_expr));
                                    } else {
                                        w.line(&format!("result[\"{}\"] = val;", variant_name));
                                    }
                                } else {
                                    // Regular struct variant: deserialize the nested object
                                    w.line("const inner: any = {};");
                                    w.line("for (const [innerKey, innerVal] of Object.entries(val as any)) {");
                                    w.indent();

                                    // Check if any fields need deserialization
                                    let has_deserializable_fields = obj.fields.iter().any(|f| needs_serialization(&f.r#type));

                                    if has_deserializable_fields {
                                        for field in &obj.fields {
                                            let camel_name = to_camel_case(&field.name);
                                            let original_name = &field.name;

                                            if needs_serialization(&field.r#type) {
                                                w.line(&format!("if (innerKey === \"{}\") {{", original_name));
                                                w.indent();
                                                let deserializer_expr = get_deserializer_expr(&field.r#type, "innerVal as any");
                                                w.line(&format!("inner[\"{}\"] = {};", camel_name, deserializer_expr));
                                                w.line("continue;");
                                                w.dedent();
                                                w.line("}");
                                            }
                                        }
                                    }

                                    w.line("const camelKey = innerKey.replace(/_([a-z])/g, (_, m: string) => m.toUpperCase());");
                                    w.line("inner[camelKey] = innerVal;");
                                    w.dedent();
                                    w.line("}");
                                    w.line(&format!("result[\"{}\"] = inner;", variant_name));
                                }

                                w.line("continue;");
                                w.dedent();
                                w.line("}");
                            }
                        } else {
                            // Unnamed variant: handle field deserialization directly
                            if obj.fields.len() == 1 {
                                let field = &obj.fields[0];
                                let camel_key = to_camel_case(&field.name);
                                let original_key = &field.name;

                                if needs_serialization(&field.r#type) {
                                    w.line(&format!("if (key === \"{}\") {{", original_key));
                                    w.indent();
                                    let deserializer_expr = get_deserializer_expr(&field.r#type, "val as any");
                                    w.line(&format!("result[\"{}\"] = {};", camel_key, deserializer_expr));
                                    w.line("continue;");
                                    w.dedent();
                                    w.line("}");
                                }
                            }
                        }
                    }
                }

                // Fallback: simple key transformation
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
        &format!("export function {}(value: {}): any {{", func_name, enum_type.name),
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
        &format!("export function {}(value: any): {} {{", func_name, enum_type.name),
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
