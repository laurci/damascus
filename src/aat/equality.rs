use super::types::*;

/// Check if two NamedTypes are structurally equal (ignoring their names)
pub fn types_are_structurally_equal(a: &NamedType, b: &NamedType) -> bool {
    match (a, b) {
        (NamedType::Object(a_obj), NamedType::Object(b_obj)) => {
            objects_are_equal(a_obj, b_obj)
        }
        (NamedType::Union(a_union), NamedType::Union(b_union)) => {
            unions_are_equal(a_union, b_union)
        }
        (NamedType::Enum(a_enum), NamedType::Enum(b_enum)) => {
            enums_are_equal(a_enum, b_enum)
        }
        _ => false, // Different variants are not equal
    }
}

fn objects_are_equal(a: &ObjectType, b: &ObjectType) -> bool {
    if a.fields.len() != b.fields.len() {
        return false;
    }

    // Fields must be in the same order with same names and types
    a.fields.iter().zip(b.fields.iter()).all(|(a_field, b_field)| {
        a_field.name == b_field.name
            && field_types_are_equal(&a_field.r#type, &b_field.r#type)
            && constraints_are_equal(&a_field.constraints, &b_field.constraints)
    })
}

fn unions_are_equal(a: &UnionType, b: &UnionType) -> bool {
    if a.variants.len() != b.variants.len() {
        return false;
    }

    // Check discriminators
    match (&a.discriminator, &b.discriminator) {
        (None, None) => {},
        (Some(a_disc), Some(b_disc)) => {
            if a_disc.property_name != b_disc.property_name {
                return false;
            }
            // Compare mappings
            match (&a_disc.mapping, &b_disc.mapping) {
                (None, None) => {},
                (Some(a_map), Some(b_map)) => {
                    if a_map != b_map {
                        return false;
                    }
                },
                _ => return false,
            }
        },
        _ => return false,
    }

    // Check variants
    a.variants.iter().zip(b.variants.iter()).all(|(a_var, b_var)| {
        a_var.name == b_var.name && variant_modes_are_equal(&a_var.mode, &b_var.mode)
    })
}

fn variant_modes_are_equal(a: &UnionTypeVariantMode, b: &UnionTypeVariantMode) -> bool {
    match (a, b) {
        (UnionTypeVariantMode::Object(a_obj), UnionTypeVariantMode::Object(b_obj)) => {
            objects_are_equal(a_obj, b_obj)
        }
        (UnionTypeVariantMode::Literal(a_lit), UnionTypeVariantMode::Literal(b_lit)) => {
            literals_are_equal(a_lit, b_lit)
        }
        _ => false,
    }
}

fn enums_are_equal(a: &EnumType, b: &EnumType) -> bool {
    if a.variants.len() != b.variants.len() {
        return false;
    }

    a.variants.iter().zip(b.variants.iter()).all(|(a_var, b_var)| {
        literals_are_equal(&a_var.value, &b_var.value)
            && a_var.description == b_var.description
    })
}

fn field_types_are_equal(a: &FieldType, b: &FieldType) -> bool {
    match (a, b) {
        (FieldType::Primitive(a_prim), FieldType::Primitive(b_prim)) => {
            primitives_are_equal(a_prim, b_prim)
        }
        (FieldType::Literal(a_lit), FieldType::Literal(b_lit)) => {
            literals_are_equal(a_lit, b_lit)
        }
        (FieldType::Optional(a_inner), FieldType::Optional(b_inner)) => {
            field_types_are_equal(a_inner, b_inner)
        }
        (FieldType::List(a_inner), FieldType::List(b_inner)) => {
            field_types_are_equal(a_inner, b_inner)
        }
        (FieldType::Map(a_inner), FieldType::Map(b_inner)) => {
            field_types_are_equal(a_inner, b_inner)
        }
        (FieldType::Reference(a_ref), FieldType::Reference(b_ref)) => {
            a_ref == b_ref
        }
        (FieldType::Intersection(a_types), FieldType::Intersection(b_types)) => {
            a_types.len() == b_types.len()
                && a_types.iter().zip(b_types.iter()).all(|(a, b)| field_types_are_equal(a, b))
        }
        (FieldType::Any, FieldType::Any) => true,
        _ => false,
    }
}

fn primitives_are_equal(a: &PrimitiveType, b: &PrimitiveType) -> bool {
    match (a, b) {
        (PrimitiveType::Bool, PrimitiveType::Bool) => true,
        (PrimitiveType::Int, PrimitiveType::Int) => true,
        (PrimitiveType::Float, PrimitiveType::Float) => true,
        (PrimitiveType::String(a_fmt), PrimitiveType::String(b_fmt)) => {
            a_fmt == b_fmt
        }
        _ => false,
    }
}

fn literals_are_equal(a: &LiteralType, b: &LiteralType) -> bool {
    match (a, b) {
        (LiteralType::String(a_s), LiteralType::String(b_s)) => a_s == b_s,
        (LiteralType::Int(a_i), LiteralType::Int(b_i)) => a_i == b_i,
        (LiteralType::Float(a_f), LiteralType::Float(b_f)) => {
            // For floats, we need to handle NaN and use approximate equality
            if a_f.is_nan() && b_f.is_nan() {
                true
            } else {
                a_f == b_f
            }
        }
        (LiteralType::Bool(a_b), LiteralType::Bool(b_b)) => a_b == b_b,
        (LiteralType::Null, LiteralType::Null) => true,
        _ => false,
    }
}

fn constraints_are_equal(a: &Option<Constraints>, b: &Option<Constraints>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a_c), Some(b_c)) => {
            a_c.minimum == b_c.minimum
                && a_c.maximum == b_c.maximum
                && a_c.exclusive_minimum == b_c.exclusive_minimum
                && a_c.exclusive_maximum == b_c.exclusive_maximum
                && a_c.multiple_of == b_c.multiple_of
                && a_c.min_length == b_c.min_length
                && a_c.max_length == b_c.max_length
                && a_c.pattern == b_c.pattern
                && a_c.min_items == b_c.min_items
                && a_c.max_items == b_c.max_items
                && a_c.unique_items == b_c.unique_items
        }
        _ => false,
    }
}
