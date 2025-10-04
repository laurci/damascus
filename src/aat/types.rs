use anyhow::Result;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Service {
    pub name: String,
    pub endpoints: Vec<Endpoint>,
}

#[derive(Debug, Clone)]
pub struct Endpoint {
    pub name: String,
    pub method: HttpMethod,
    pub path: Vec<PathSegment>,
    pub query: Option<FieldType>,
    pub body: Option<FieldType>,
    pub response: FieldType,
}

#[derive(Debug, Clone)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
}

#[derive(Debug, Clone)]
pub enum PathSegment {
    Literal(String),
    Parameter { name: String, r#type: FieldType },
}

#[derive(Debug, Clone)]
pub enum FieldType {
    Primitive(PrimitiveType),
    Literal(LiteralType),
    Optional(Box<FieldType>),
    List(Box<FieldType>),
    Map(Box<FieldType>),
    Reference(String),
    Intersection(Vec<FieldType>),
    Any,
}

#[derive(Debug, Clone)]
pub enum PrimitiveType {
    Bool,
    Int,
    Float,
    String(Option<StringFormat>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum StringFormat {
    DateTime,
    Date,
    Time,
    Uuid,
    Email,
    Uri,
    Hostname,
    Ipv4,
    Ipv6,
}

#[derive(Debug, Clone)]
pub enum LiteralType {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Null,
}

#[derive(Debug, Clone)]
pub enum NamedType {
    Object(ObjectType),
    Union(UnionType),
    Enum(EnumType),
}

#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub r#type: Box<FieldType>,
    pub constraints: Option<Constraints>,
}

#[derive(Debug, Clone)]
pub struct Constraints {
    // Numeric constraints (minimum and exclusive_minimum are mutually exclusive)
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
    pub exclusive_minimum: Option<f64>,
    pub exclusive_maximum: Option<f64>,
    pub multiple_of: Option<f64>,

    // String constraints
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
    pub pattern: Option<String>,

    // Array constraints
    pub min_items: Option<usize>,
    pub max_items: Option<usize>,
    pub unique_items: Option<bool>,
}

impl Constraints {
    /// Validate that constraints are logically consistent
    pub fn validate(&self) -> Result<()> {
        // Validate numeric constraints
        if let (Some(min), Some(max)) = (self.minimum, self.maximum) {
            if min > max {
                anyhow::bail!("Invalid constraint: minimum ({}) must be <= maximum ({})", min, max);
            }
        }
        if let (Some(min), Some(max)) = (self.exclusive_minimum, self.exclusive_maximum) {
            if min >= max {
                anyhow::bail!("Invalid constraint: exclusive_minimum ({}) must be < exclusive_maximum ({})", min, max);
            }
        }
        // Cross-checks between exclusive and inclusive bounds
        if let (Some(min), Some(ex_max)) = (self.minimum, self.exclusive_maximum) {
            if min >= ex_max {
                anyhow::bail!("Invalid constraint: minimum ({}) must be < exclusive_maximum ({})", min, ex_max);
            }
        }
        if let (Some(ex_min), Some(max)) = (self.exclusive_minimum, self.maximum) {
            if ex_min >= max {
                anyhow::bail!("Invalid constraint: exclusive_minimum ({}) must be < maximum ({})", ex_min, max);
            }
        }

        // Validate string constraints
        if let (Some(min_len), Some(max_len)) = (self.min_length, self.max_length) {
            if min_len > max_len {
                anyhow::bail!("Invalid constraint: minLength ({}) must be <= maxLength ({})", min_len, max_len);
            }
        }

        // Validate array constraints
        if let (Some(min_items), Some(max_items)) = (self.min_items, self.max_items) {
            if min_items > max_items {
                anyhow::bail!("Invalid constraint: minItems ({}) must be <= maxItems ({})", min_items, max_items);
            }
        }

        // Validate multipleOf is positive
        if let Some(multiple) = self.multiple_of {
            if multiple <= 0.0 {
                anyhow::bail!("Invalid constraint: multipleOf ({}) must be > 0", multiple);
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ObjectType {
    pub name: String,
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone)]
pub struct UnionType {
    pub name: String,
    pub discriminator: Option<Discriminator>,
    pub variants: Vec<UnionTypeVariant>,
}

#[derive(Debug, Clone)]
pub struct Discriminator {
    pub property_name: String,
    pub mapping: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone)]
pub struct UnionTypeVariant {
    pub name: Option<String>,
    pub mode: Box<UnionTypeVariantMode>,
}

#[derive(Debug, Clone)]
pub enum UnionTypeVariantMode {
    Object(ObjectType),
    Literal(LiteralType),
}

#[derive(Debug, Clone)]
pub struct EnumType {
    pub name: String,
    pub variants: Vec<EnumVariant>,
}

#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub value: LiteralType,
    pub description: Option<String>,
}
