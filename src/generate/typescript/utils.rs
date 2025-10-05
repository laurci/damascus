use crate::aat::*;

/// Convert field type to TypeScript type string
pub fn field_type_to_ts(field_type: &FieldType) -> String {
    match field_type {
        FieldType::Primitive(prim) => primitive_to_ts(prim),
        FieldType::Literal(lit) => literal_to_ts(lit),
        FieldType::Optional(inner) => format!("{} | undefined", field_type_to_ts(inner)),
        FieldType::List(inner) => format!("{}[]", field_type_to_ts(inner)),
        FieldType::Map(inner) => {
            format!("{{ [key: string]: {} }}", field_type_to_ts(inner))
        }
        FieldType::Stream(inner) => {
            let inner_ts = field_type_to_ts(inner);
            format!("WebSocketStream<{}>", inner_ts)
        }
        FieldType::Reference(name) => name.clone(),
        FieldType::Intersection(types) => types
            .iter()
            .map(|t| field_type_to_ts(t))
            .collect::<Vec<_>>()
            .join(" & "),
        FieldType::Tuple(types) => {
            let inner = types
                .iter()
                .map(|t| field_type_to_ts(t))
                .collect::<Vec<_>>()
                .join(", ");
            format!("[{}]", inner)
        }
        FieldType::Any => "any".to_string(),
    }
}

pub fn primitive_to_ts(prim: &PrimitiveType) -> String {
    match prim {
        PrimitiveType::Bool => "boolean".to_string(),
        PrimitiveType::Int => "number".to_string(),
        PrimitiveType::Float => "number".to_string(),
        PrimitiveType::String(_) => "string".to_string(),
    }
}

pub fn literal_to_ts(lit: &LiteralType) -> String {
    match lit {
        LiteralType::String(s) => format!("\"{}\"", s),
        LiteralType::Int(i) => i.to_string(),
        LiteralType::Float(f) => f.to_string(),
        LiteralType::Bool(b) => b.to_string(),
        LiteralType::Null => "null".to_string(),
    }
}

pub fn literal_to_ts_with_camel(lit: &LiteralType) -> String {
    match lit {
        LiteralType::String(s) => format!("\"{}\"", to_camel_case(s)),
        LiteralType::Int(i) => i.to_string(),
        LiteralType::Float(f) => f.to_string(),
        LiteralType::Bool(b) => b.to_string(),
        LiteralType::Null => "null".to_string(),
    }
}

pub fn to_pascal_case(s: &str) -> String {
    s.split(&['-', '_'][..])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect()
}

pub fn to_camel_case(s: &str) -> String {
    let parts: Vec<&str> = s.split(&['-', '_'][..]).filter(|p| !p.is_empty()).collect();
    if parts.is_empty() {
        return String::new();
    }

    let mut result = parts[0].to_lowercase();
    for part in &parts[1..] {
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            result.push_str(&first.to_uppercase().collect::<String>());
            result.push_str(&chars.collect::<String>().to_lowercase());
        }
    }
    result
}

/// Checks if a string is a valid TypeScript identifier
/// Returns true if it can be used unquoted as a property name
pub fn is_valid_ts_identifier(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    let mut chars = s.chars();

    // First character must be letter, underscore, or dollar sign
    match chars.next() {
        Some(c) if c.is_alphabetic() || c == '_' || c == '$' => {}
        _ => return false,
    }

    // Remaining characters must be alphanumeric, underscore, or dollar sign
    for c in chars {
        if !c.is_alphanumeric() && c != '_' && c != '$' {
            return false;
        }
    }

    true
}

/// Quotes a string if it's not a valid TypeScript identifier
pub fn quote_if_needed(s: &str) -> String {
    if is_valid_ts_identifier(s) {
        s.to_string()
    } else {
        format!("\"{}\"", s)
    }
}
