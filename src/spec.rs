use std::collections::BTreeMap;

use schemars::{JsonSchema, Schema, schema_for};

#[derive(Debug, Clone)]
pub struct Spec {
    name: String,
    organization: Option<String>,
    repository: Option<String>,
    website: Option<String>,
    docs: Option<String>,
    description: Option<String>,
    headers: BTreeMap<String, HeaderValue>,
    services: Vec<Service>,
}

impl Spec {
    pub fn new(name: impl AsRef<str>) -> Self {
        let name_str = name.as_ref();
        assert!(!name_str.trim().is_empty(), "Spec name cannot be empty");
        Self {
            name: name_str.to_string(),
            organization: None,
            repository: None,
            website: None,
            docs: None,
            description: None,
            headers: BTreeMap::new(),
            services: vec![],
        }
    }

    pub fn services(&self) -> &[Service] {
        &self.services
    }

    pub fn headers(&self) -> &BTreeMap<String, HeaderValue> {
        &self.headers
    }

    pub fn header(mut self, name: impl AsRef<str>, value: HeaderValue) -> Self {
        self.headers.insert(name.as_ref().to_string(), value);
        self
    }

    pub fn organization(mut self, organization: impl AsRef<str>) -> Self {
        self.organization = Some(organization.as_ref().to_string());
        self
    }

    pub fn repository(mut self, repository: impl AsRef<str>) -> Self {
        self.repository = Some(repository.as_ref().to_string());
        self
    }

    pub fn website(mut self, website: impl AsRef<str>) -> Self {
        self.website = Some(website.as_ref().to_string());
        self
    }

    pub fn docs(mut self, docs: impl AsRef<str>) -> Self {
        self.docs = Some(docs.as_ref().to_string());
        self
    }

    pub fn description(mut self, description: impl AsRef<str>) -> Self {
        self.description = Some(description.as_ref().to_string());
        self
    }

    pub fn service<F: FnOnce(Service) -> Service>(
        mut self,
        name: impl AsRef<str>,
        block: F,
    ) -> Self {
        let service = Service::new(name);
        let service = block(service);
        self.services.push(service);
        self
    }
}

#[derive(Debug, Clone)]
pub struct Service {
    name: String,
    endpoints: Vec<Endpoint>,
    headers: BTreeMap<String, HeaderValue>,
}

impl Service {
    pub fn new(name: impl AsRef<str>) -> Self {
        let name_str = name.as_ref();
        assert!(!name_str.trim().is_empty(), "Service name cannot be empty");
        Self {
            name: name_str.to_string(),
            endpoints: vec![],
            headers: BTreeMap::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn endpoints(&self) -> &[Endpoint] {
        &self.endpoints
    }

    pub fn headers(&self) -> &BTreeMap<String, HeaderValue> {
        &self.headers
    }

    pub fn header(mut self, name: impl AsRef<str>, value: HeaderValue) -> Self {
        self.headers.insert(name.as_ref().to_string(), value);
        self
    }

    pub fn endpoint<F: FnOnce(Endpoint) -> Endpoint>(
        mut self,
        name: impl AsRef<str>,
        method: Method,
        path: Vec<PathSegment>,
        block: F,
    ) -> Self {
        let endpoint = Endpoint::new(name, method, path);
        let endpoint = block(endpoint);
        self.endpoints.push(endpoint);
        self
    }

    pub fn get<F: FnOnce(Endpoint) -> Endpoint>(
        self,
        name: impl AsRef<str>,
        path: Vec<PathSegment>,
        block: F,
    ) -> Self {
        self.endpoint(name, Method::Get, path, block)
    }

    pub fn post<F: FnOnce(Endpoint) -> Endpoint>(
        self,
        name: impl AsRef<str>,
        path: Vec<PathSegment>,
        block: F,
    ) -> Self {
        self.endpoint(name, Method::Post, path, block)
    }

    pub fn put<F: FnOnce(Endpoint) -> Endpoint>(
        self,
        name: impl AsRef<str>,
        path: Vec<PathSegment>,
        block: F,
    ) -> Self {
        self.endpoint(name, Method::Put, path, block)
    }

    pub fn delete<F: FnOnce(Endpoint) -> Endpoint>(
        self,
        name: impl AsRef<str>,
        path: Vec<PathSegment>,
        block: F,
    ) -> Self {
        self.endpoint(name, Method::Delete, path, block)
    }

    pub fn patch<F: FnOnce(Endpoint) -> Endpoint>(
        self,
        name: impl AsRef<str>,
        path: Vec<PathSegment>,
        block: F,
    ) -> Self {
        self.endpoint(name, Method::Patch, path, block)
    }
}

#[derive(Debug, Clone)]
pub struct Endpoint {
    name: String,
    method: Method,
    path: Vec<PathSegment>,
    query: Option<Type>,
    body: Option<Type>,
    response: Type,
    upgrade: Option<Upgrade>,
    headers: BTreeMap<String, HeaderValue>,
}

#[derive(Debug, Clone)]
pub enum Method {
    Get,
    Post,
    Put,
    Delete,
    Patch,
}

#[derive(Debug, Clone)]
pub enum Upgrade {
    Ws,
}

#[derive(Debug, Clone)]
pub enum PathSegment {
    Literal(String),
    Type { name: String, r#type: Type },
}

#[derive(Debug, Clone)]
pub enum HeaderValue {
    Literal(String),
    Type {
        name: String,
        r#type: Type,
    },
    Pattern {
        pattern: String,
        name: String,
        r#type: Type,
    },
}

impl Endpoint {
    pub fn new(name: impl AsRef<str>, method: Method, path: Vec<PathSegment>) -> Self {
        let name_str = name.as_ref();
        assert!(!name_str.trim().is_empty(), "Endpoint name cannot be empty");
        Self {
            name: name_str.to_string(),
            method,
            path,
            query: None,
            body: None,
            response: Type::Void,
            upgrade: None,
            headers: BTreeMap::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn method(&self) -> &Method {
        &self.method
    }

    pub fn path(&self) -> &[PathSegment] {
        &self.path
    }

    pub fn query_type(&self) -> Option<&Type> {
        self.query.as_ref()
    }

    pub fn body_type(&self) -> Option<&Type> {
        self.body.as_ref()
    }

    pub fn response_type(&self) -> &Type {
        &self.response
    }

    pub fn upgrade_type(&self) -> Option<&Upgrade> {
        self.upgrade.as_ref()
    }

    pub fn headers(&self) -> &BTreeMap<String, HeaderValue> {
        &self.headers
    }

    pub fn header(mut self, name: impl AsRef<str>, value: HeaderValue) -> Self {
        self.headers.insert(name.as_ref().to_string(), value);
        self
    }

    pub fn response(mut self, t: Type) -> Self {
        self.response = t;
        self
    }

    pub fn query(mut self, t: Type) -> Self {
        self.query = Some(t);
        self
    }

    pub fn body(mut self, t: Type) -> Self {
        self.body = Some(t);
        self
    }

    pub fn upgrade(mut self, upgrade: Upgrade) -> Self {
        self.upgrade = Some(upgrade);
        self
    }
}

#[derive(Clone, Debug)]
pub enum Type {
    Void,
    Schema(schemars::Schema),
    Stream(Box<Type>),
    List(Box<Type>),
    Optional(Box<Type>),
    Tuple(Vec<Type>),
    NamedTuple(BTreeMap<String, Type>),
}

impl Type {
    pub fn void() -> Self {
        Type::Void
    }

    pub fn schema<T: JsonSchema>() -> Self {
        let s: Schema = schema_for!(T);
        Type::Schema(s)
    }

    pub fn stream(t: Type) -> Self {
        Type::Stream(Box::new(t))
    }

    pub fn list(t: Type) -> Self {
        Type::List(Box::new(t))
    }

    pub fn optional(t: Type) -> Self {
        Type::Optional(Box::new(t))
    }

    pub fn tuple(t: Vec<Type>) -> Self {
        Type::Tuple(t)
    }

    pub fn named_tuple(t: BTreeMap<String, Type>) -> Self {
        Type::NamedTuple(t)
    }

    pub fn wrap_stream(&self) -> Self {
        Type::Stream(Box::new(self.clone()))
    }

    pub fn wrap_list(&self) -> Self {
        Type::List(Box::new(self.clone()))
    }

    pub fn wrap_optional(&self) -> Self {
        Type::Optional(Box::new(self.clone()))
    }
}

pub mod default {
    use super::*;

    pub fn endpoint(endpoint: Endpoint) -> Endpoint {
        endpoint
    }

    pub fn service(service: Service) -> Service {
        service
    }
}
