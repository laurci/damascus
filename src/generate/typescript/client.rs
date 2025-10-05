use crate::aat::*;
use crate::generate::writer::CodeWriter;
use anyhow::Result;

use super::utils::*;

pub fn generate_api_client(writer: &mut CodeWriter, aat: &AAT) -> Result<()> {
    // Generate ClientConfig interface
    writer.block("export interface ClientConfig {", "}", |w| {
        w.line("baseUrl: string;");

        // Add root-level header parameters to config
        for header in &aat.headers {
            match &header.value {
                HeaderValue::Parameter { name, field_type } => {
                    let ts_type = field_type_to_ts(field_type);
                    let is_optional = matches!(field_type, FieldType::Optional(_));
                    if is_optional {
                        w.line(&format!("{}?: {};", name, ts_type));
                    } else {
                        w.line(&format!("{}: {};", name, ts_type));
                    }
                }
                HeaderValue::Pattern {
                    param_name,
                    field_type,
                    ..
                } => {
                    let ts_type = field_type_to_ts(field_type);
                    let is_optional = matches!(field_type, FieldType::Optional(_));
                    if is_optional {
                        w.line(&format!("{}?: {};", param_name, ts_type));
                    } else {
                        w.line(&format!("{}: {};", param_name, ts_type));
                    }
                }
                HeaderValue::Literal(_) => {
                    // Literals don't need parameters
                }
            }
        }

        w.line("options?: RequestInit;");
        w.line("fetchImpl?: typeof fetch;");
        w.line("WebSocketImpl?: typeof WebSocket;");
    });
    writer.empty_line();

    // Generate Client class
    writer.block("export class Client {", "}", |w| {
        // Build root-level header storage fields
        let mut root_header_storage = Vec::new();
        for header in &aat.headers {
            match &header.value {
                HeaderValue::Parameter { name, .. } => {
                    root_header_storage.push(name.clone());
                }
                HeaderValue::Pattern {
                    param_name,
                    ..
                } => {
                    root_header_storage.push(param_name.clone());
                }
                HeaderValue::Literal(_) => {}
            }
        }

        // Declare private fields
        w.line("private readonly baseUrl: string;");
        for name in &root_header_storage {
            w.line(&format!("private readonly rootHeader_{}: any;", name));
        }
        w.line("private readonly options?: RequestInit;");
        w.line("private readonly fetchImpl: typeof fetch;");
        w.line("private readonly WebSocketImpl: typeof WebSocket;");
        w.empty_line();

        // Constructor
        w.block("constructor(config: ClientConfig) {", "}", |w| {
            w.line("this.baseUrl = config.baseUrl;");
            // Store root header values
            for name in &root_header_storage {
                w.line(&format!("this.rootHeader_{} = config.{};", name, name));
            }
            w.line("this.options = config.options;");
            w.line("this.fetchImpl = config.fetchImpl || globalThis.fetch;");
            w.line("this.WebSocketImpl = config.WebSocketImpl || globalThis.WebSocket;");
        });
        w.empty_line();

        // Generate service factory methods
        for service in &aat.services {
            let class_name = to_pascal_case(&service.name);

            // Build service-level header parameters
            let mut service_header_params = Vec::new();
            for header in &service.headers {
                match &header.value {
                    HeaderValue::Parameter { name, field_type } => {
                        let ts_type = field_type_to_ts(field_type);
                        let is_optional = matches!(field_type, FieldType::Optional(_));
                        if is_optional {
                            service_header_params.push(format!("{}?: {}", name, ts_type));
                        } else {
                            service_header_params.push(format!("{}: {}", name, ts_type));
                        }
                    }
                    HeaderValue::Pattern {
                        param_name,
                        field_type,
                        ..
                    } => {
                        let ts_type = field_type_to_ts(field_type);
                        let is_optional = matches!(field_type, FieldType::Optional(_));
                        if is_optional {
                            service_header_params.push(format!("{}?: {}", param_name, ts_type));
                        } else {
                            service_header_params.push(format!("{}: {}", param_name, ts_type));
                        }
                    }
                    HeaderValue::Literal(_) => {
                        // Literals don't need parameters
                    }
                }
            }

            let method_params = if service_header_params.is_empty() {
                String::new()
            } else {
                service_header_params.join(", ")
            };

            let getter = if method_params.is_empty() {
                String::from("get ")
            } else {
                String::new()
            };

            // Create service factory method
            w.line(&format!(
                "{}{}({}): {}Client {{",
                getter, service.name, method_params, class_name
            ));
            w.indent();

            // Build root headers object
            w.line("const rootHeaders: Record<string, string> = {};");
            for header in &aat.headers {
                match &header.value {
                    HeaderValue::Literal(value) => {
                        w.line(&format!("rootHeaders['{}'] = '{}';", header.name, value));
                    }
                    HeaderValue::Parameter { name, .. } => {
                        w.line(&format!(
                            "rootHeaders['{}'] = String(this.rootHeader_{});",
                            header.name, name
                        ));
                    }
                    HeaderValue::Pattern {
                        pattern,
                        param_name,
                        ..
                    } => {
                        // Replace {param_name} in pattern with the parameter value
                        let placeholder = format!("{{{}}}", param_name);
                        let pattern_expr = pattern.replace(
                            &placeholder,
                            &format!("${{String(this.rootHeader_{})}}", param_name),
                        );
                        w.line(&format!(
                            "rootHeaders['{}'] = `{}`;",
                            header.name, pattern_expr
                        ));
                    }
                }
            }

            // Build service headers object
            w.line("const serviceHeaders: Record<string, string> = {};");
            for header in &service.headers {
                match &header.value {
                    HeaderValue::Literal(value) => {
                        w.line(&format!("serviceHeaders['{}'] = '{}';", header.name, value));
                    }
                    HeaderValue::Parameter { name, .. } => {
                        w.line(&format!(
                            "serviceHeaders['{}'] = String({});",
                            header.name, name
                        ));
                    }
                    HeaderValue::Pattern {
                        pattern,
                        param_name,
                        ..
                    } => {
                        let placeholder = format!("{{{}}}", param_name);
                        let pattern_expr =
                            pattern.replace(&placeholder, &format!("${{String({})}}", param_name));
                        w.line(&format!(
                            "serviceHeaders['{}'] = `{}`;",
                            header.name, pattern_expr
                        ));
                    }
                }
            }

            w.line(&format!(
                "return new {}Client(this.baseUrl, rootHeaders, serviceHeaders, this.options, this.fetchImpl, this.WebSocketImpl);",
                class_name
            ));
            w.dedent();
            w.line("}");
            w.empty_line();
        }
    });
    Ok(())
}

pub fn generate_service(writer: &mut CodeWriter, service: &Service) -> Result<()> {
    // Pre-generate all endpoint methods to avoid borrowing issues
    let mut methods = Vec::new();
    for endpoint in &service.endpoints {
        methods.push(generate_endpoint_method_str(endpoint)?);
    }

    let class_name = to_pascal_case(&service.name);
    writer
        .block(&format!("class {}Client {{", class_name), "}", |w| {
            // Constructor
            w.line("constructor(private baseUrl: string, private rootHeaders: Record<string, string>, private serviceHeaders: Record<string, string>, private options: RequestInit | undefined, private fetchImpl: typeof fetch, private WebSocketImpl: typeof WebSocket) {}");
            w.empty_line();

            // Write pre-generated methods
            for method_str in methods {
                for line in method_str.lines() {
                    w.line(line);
                }
                w.empty_line();
            }
        });
    Ok(())
}

fn generate_endpoint_method_str(endpoint: &Endpoint) -> Result<String> {
    let mut w = CodeWriter::new();
    generate_endpoint_method_inner(&mut w, endpoint)?;
    Ok(w.into_string())
}

fn generate_endpoint_method_inner(w: &mut CodeWriter, endpoint: &Endpoint) -> Result<()> {
    let method_name = to_camel_case(&endpoint.name);
    let is_websocket = matches!(endpoint.upgrade, Some(Upgrade::Ws));

    // Build parameter lists - separate required and optional to maintain correct order
    let mut required_params = Vec::new();
    let mut optional_params = Vec::new();

    // Endpoint-level header parameters
    for header in &endpoint.headers {
        match &header.value {
            HeaderValue::Parameter { name, field_type } => {
                let ts_type = field_type_to_ts(field_type);
                let is_optional = matches!(field_type, FieldType::Optional(_));
                if is_optional {
                    optional_params.push(format!("{}?: {}", name, ts_type));
                } else {
                    required_params.push(format!("{}: {}", name, ts_type));
                }
            }
            HeaderValue::Pattern {
                param_name,
                field_type,
                ..
            } => {
                let ts_type = field_type_to_ts(field_type);
                let is_optional = matches!(field_type, FieldType::Optional(_));
                if is_optional {
                    optional_params.push(format!("{}?: {}", param_name, ts_type));
                } else {
                    required_params.push(format!("{}: {}", param_name, ts_type));
                }
            }
            HeaderValue::Literal(_) => {
                // Literals don't need parameters
            }
        }
    }

    // Path parameters
    for segment in &endpoint.path {
        if let PathSegment::Parameter { name, r#type } = segment {
            let ts_type = field_type_to_ts(r#type);
            let is_optional = matches!(r#type, FieldType::Optional(_));
            if is_optional {
                optional_params.push(format!("{}?: {}", name, ts_type));
            } else {
                required_params.push(format!("{}: {}", name, ts_type));
            }
        }
    }

    // Query parameters
    if let Some(query_type) = &endpoint.query {
        let ts_type = field_type_to_ts(query_type);
        let is_optional = matches!(query_type, FieldType::Optional(_));
        if is_optional {
            optional_params.push(format!("query?: {}", ts_type));
        } else {
            required_params.push(format!("query: {}", ts_type));
        }
    }

    // Body parameter
    if let Some(body_type) = &endpoint.body {
        let ts_type = field_type_to_ts(body_type);
        let is_optional = matches!(body_type, FieldType::Optional(_));
        if is_optional {
            optional_params.push(format!("body?: {}", ts_type));
        } else {
            required_params.push(format!("body: {}", ts_type));
        }
    }

    // Combine parameters: required first, then optional
    let mut params = required_params;
    params.extend(optional_params);

    let params_str = params.join(", ");
    let is_void_response = matches!(endpoint.response, FieldType::Any);
    let return_type = if is_void_response {
        "void".to_string()
    } else {
        field_type_to_ts(&endpoint.response)
    };

    // For WebSocket endpoints, return the stream directly without Promise wrapper
    let method_signature = if is_websocket {
        format!("{}({}): {}", method_name, params_str, return_type)
    } else {
        format!("async {}({}): Promise<{}>", method_name, params_str, return_type)
    };

    w.block(&format!("{} {{", method_signature), "}", |w| {
        // Build endpoint-level headers
        w.line("const endpointHeaders: Record<string, string> = {};");
        for header in &endpoint.headers {
            match &header.value {
                HeaderValue::Literal(value) => {
                    w.line(&format!("endpointHeaders['{}'] = '{}';", header.name, value));
                }
                HeaderValue::Parameter { name, .. } => {
                    w.line(&format!("endpointHeaders['{}'] = String({});", header.name, name));
                }
                HeaderValue::Pattern { pattern, param_name, .. } => {
                    let placeholder = format!("{{{}}}", param_name);
                    let pattern_expr = pattern.replace(&placeholder, &format!("${{String({})}}", param_name));
                    w.line(&format!("endpointHeaders['{}'] = `{}`;", header.name, pattern_expr));
                }
            }
        }

        // Merge all headers
        w.line("const mergedHeaders = { ...this.rootHeaders, ...this.serviceHeaders, ...endpointHeaders, ...this.options?.headers };");
        w.empty_line();

        // Build path
        let mut path_str = String::new();
        for segment in &endpoint.path {
            match segment {
                PathSegment::Literal(lit) => path_str.push_str(&format!("/{}", lit)),
                PathSegment::Parameter { name, .. } => path_str.push_str(&format!("/${{{}}}", name)),
            }
        }
        w.line(&format!("const path = `{}`;", path_str));

        // Build URL with query params
        if let Some(query_type) = &endpoint.query {
            // Serialize query if needed
            if needs_serialization(query_type) {
                let serializer = get_serializer_call(query_type);
                w.line(&format!("const serializedQuery = {}(query);", serializer));
                w.line("const params = new URLSearchParams();");
                w.block("for (const [key, value] of Object.entries(serializedQuery)) {", "}", |w| {
                    w.block("if (value !== undefined && value !== null) {", "}", |w| {
                        w.line("params.append(key, String(value));");
                    });
                });
            } else {
                w.line("const params = new URLSearchParams();");
                w.block("for (const [key, value] of Object.entries(query)) {", "}", |w| {
                    w.block("if (value !== undefined && value !== null) {", "}", |w| {
                        w.line("params.append(key, String(value));");
                    });
                });
            }
            w.line("const url = `${this.baseUrl}${path}?${params.toString()}`;");
        } else {
            w.line("const url = `${this.baseUrl}${path}`;");
        }

        if is_websocket {
            // Generate WebSocket connection code
            // Extract the inner type from Stream<T>
            if let FieldType::Stream(inner_type) = &endpoint.response {
                let deserializer = if needs_serialization(inner_type) {
                    get_deserializer_call(inner_type)
                } else {
                    "(data: any) => data".to_string()
                };
                w.line(&format!("const stream = new WebSocketStream(url, {}, mergedHeaders, this.WebSocketImpl);", deserializer));
                w.line("return stream;");
            }
        } else {
            // Generate regular HTTP request code
            let http_method = match endpoint.method {
                HttpMethod::Get => "GET",
                HttpMethod::Post => "POST",
                HttpMethod::Put => "PUT",
                HttpMethod::Delete => "DELETE",
                HttpMethod::Patch => "PATCH",
            };

            // Serialize body if needed
            if let Some(body_type) = &endpoint.body {
                if needs_serialization(body_type) {
                    let serializer = get_serializer_call(body_type);
                    w.line(&format!("const serializedBody = {}(body);", serializer));
                }
            }

            w.block("const response = await this.fetchImpl(url, {", "});", |w| {
                w.line(&format!("method: '{}',", http_method));
                if endpoint.body.is_some() {
                    w.line("headers: { 'Content-Type': 'application/json', ...mergedHeaders },");
                    if needs_serialization(endpoint.body.as_ref().unwrap()) {
                        w.line("body: JSON.stringify(serializedBody),");
                    } else {
                        w.line("body: JSON.stringify(body),");
                    }
                } else {
                    w.line("headers: mergedHeaders,");
                }
                w.line("...this.options,");
            });

            w.empty_line();
            w.block("if (!response.ok) {", "}", |w| {
                w.line("throw new Error(`HTTP error! status: ${response.status}`);");
            });

            // Handle void response type
            if is_void_response {
                // Don't return anything for void
            } else {
                w.empty_line();
                // Deserialize response if needed
                if needs_serialization(&endpoint.response) {
                    let deserializer = get_deserializer_call(&endpoint.response);
                    w.line("const data = await response.json();");
                    // Wrap inline lambdas in parentheses
                    if deserializer.starts_with("(v: any)") || deserializer.starts_with("(data: any)") {
                        w.line(&format!("return ({})(data);", deserializer));
                    } else {
                        w.line(&format!("return {}(data);", deserializer));
                    }
                } else {
                    w.line("return response.json();");
                }
            }
        }
    });
    Ok(())
}

fn needs_serialization(field_type: &FieldType) -> bool {
    match field_type {
        FieldType::Reference(_) => true,
        FieldType::List(inner) => needs_serialization(inner),
        FieldType::Optional(inner) => needs_serialization(inner),
        FieldType::Map(inner) => needs_serialization(inner),
        FieldType::Stream(inner) => needs_serialization(inner),
        FieldType::Tuple(types) => types.iter().any(|t| needs_serialization(t)),
        FieldType::Intersection(types) => types.iter().any(|t| needs_serialization(t)),
        _ => false,
    }
}

fn get_serializer_call(field_type: &FieldType) -> String {
    match field_type {
        FieldType::Reference(name) => format!("serialize{}", name),
        FieldType::List(inner) => {
            if needs_serialization(inner) {
                let inner_serializer = get_serializer_call(inner);
                // Wrap inline lambdas in parentheses when calling them
                if inner_serializer.starts_with("(") {
                    format!("(v: any) => v.map((x: any) => ({})(x))", inner_serializer)
                } else {
                    format!("(v: any) => v.map((x: any) => {}(x))", inner_serializer)
                }
            } else {
                "(v: any) => v".to_string()
            }
        }
        FieldType::Optional(inner) => {
            if needs_serialization(inner) {
                let inner_serializer = get_serializer_call(inner);
                // Wrap inline lambdas in parentheses when calling them
                if inner_serializer.starts_with("(") {
                    format!(
                        "(v: any) => v !== undefined ? ({})(v) : undefined",
                        inner_serializer
                    )
                } else {
                    format!(
                        "(v: any) => v !== undefined ? {}(v) : undefined",
                        inner_serializer
                    )
                }
            } else {
                "(v: any) => v".to_string()
            }
        }
        FieldType::Map(inner) => {
            if needs_serialization(inner) {
                let inner_serializer = get_serializer_call(inner);
                // Wrap inline lambdas in parentheses when calling them
                if inner_serializer.starts_with("(") {
                    format!("(v: any) => Object.fromEntries(Object.entries(v).map(([k, val]) => [k, ({})(val)]))", inner_serializer)
                } else {
                    format!("(v: any) => Object.fromEntries(Object.entries(v).map(([k, val]) => [k, {}(val)]))", inner_serializer)
                }
            } else {
                "(v: any) => v".to_string()
            }
        }
        FieldType::Tuple(types) => {
            if types.iter().any(|t| needs_serialization(t)) {
                let serializers: Vec<String> = types.iter().enumerate().map(|(i, t)| {
                    if needs_serialization(t) {
                        let serializer = get_serializer_call(t);
                        // Wrap inline lambdas in parentheses when calling them
                        if serializer.starts_with("(") {
                            format!("({})(v[{}])", serializer, i)
                        } else {
                            format!("{}(v[{}])", serializer, i)
                        }
                    } else {
                        format!("v[{}]", i)
                    }
                }).collect();
                format!("(v: any) => [{}]", serializers.join(", "))
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
                // Wrap inline lambdas in parentheses when calling them
                if inner_deserializer.starts_with("(") {
                    format!("(v: any) => v.map((x: any) => ({})(x))", inner_deserializer)
                } else {
                    format!("(v: any) => v.map((x: any) => {}(x))", inner_deserializer)
                }
            } else {
                "(v: any) => v".to_string()
            }
        }
        FieldType::Optional(inner) => {
            if needs_serialization(inner) {
                let inner_deserializer = get_deserializer_call(inner);
                // Wrap inline lambdas in parentheses when calling them
                if inner_deserializer.starts_with("(") {
                    format!(
                        "(v: any) => v !== undefined && v !== null ? ({})(v) : undefined",
                        inner_deserializer
                    )
                } else {
                    format!(
                        "(v: any) => v !== undefined && v !== null ? {}(v) : undefined",
                        inner_deserializer
                    )
                }
            } else {
                "(v: any) => v".to_string()
            }
        }
        FieldType::Map(inner) => {
            if needs_serialization(inner) {
                let inner_deserializer = get_deserializer_call(inner);
                // Wrap inline lambdas in parentheses when calling them
                if inner_deserializer.starts_with("(") {
                    format!("(v: any) => Object.fromEntries(Object.entries(v).map(([k, val]) => [k, ({})(val)]))", inner_deserializer)
                } else {
                    format!("(v: any) => Object.fromEntries(Object.entries(v).map(([k, val]) => [k, {}(val)]))", inner_deserializer)
                }
            } else {
                "(v: any) => v".to_string()
            }
        }
        FieldType::Tuple(types) => {
            if types.iter().any(|t| needs_serialization(t)) {
                let deserializers: Vec<String> = types.iter().enumerate().map(|(i, t)| {
                    if needs_serialization(t) {
                        let deserializer = get_deserializer_call(t);
                        // Wrap inline lambdas in parentheses when calling them
                        if deserializer.starts_with("(") {
                            format!("({})(v[{}])", deserializer, i)
                        } else {
                            format!("{}(v[{}])", deserializer, i)
                        }
                    } else {
                        format!("v[{}]", i)
                    }
                }).collect();
                format!("(v: any) => [{}]", deserializers.join(", "))
            } else {
                "(v: any) => v".to_string()
            }
        }
        _ => "(v: any) => v".to_string(),
    }
}
