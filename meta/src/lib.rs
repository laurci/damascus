mod header;
mod path;

use quote::quote;

use crate::{
    header::HeaderValue,
    path::{PathSegment, PathSegments},
};

/* path!("core", namespace: String, "abcd") =>
    vec! [
        PathSegment::Literal("core"),
        PathSegment::Type { name: "namespace", r#type: type_of!(String) },
        PathSegment::Literal("abcd"),
    ]
*/
#[proc_macro]
pub fn path(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let path = syn::parse_macro_input!(input as PathSegments);
    let segments = path
        .segments
        .iter()
        .map(|segment| match segment {
            PathSegment::Literal(literal) => quote! { damascus::spec::PathSegment::Literal(#literal.to_string()) },
            PathSegment::Type { name, r#type } => {
                let name = name.to_string();
                quote! { damascus::spec::PathSegment::Type { name: #name.to_string(), r#type: damascus::type_of!(#r#type) } }
            }
        })
        .collect::<Vec<_>>();

    proc_macro::TokenStream::from(quote! { vec! [#(#segments),*] })
}

/*
   header_value!("the value of the header") => damascus::spec::HeaderValue::Literal("the value of the header".to_string())
   header_value!(any_identifier) => damascus::spec::HeaderValue::Literal(any_identifier.to_string())
   header_value!(namespace: String) => damascus::spec::HeaderValue::Type{ name: "namespace".to_string(), r#type: damascus::type_of!(String) })
   header_value!("Bearer {}" use apiKey: String) => damascus::spec::HeaderValue::Pattern{ pattern: "Bearer {}".to_string(), name: "apiKey".to_string(), r#type: damascus::type_of!(String) })
*/
#[proc_macro]
pub fn header_value(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let header_value = syn::parse_macro_input!(input as HeaderValue);
    let token_stream = match header_value {
        HeaderValue::Literal(lit_str) => proc_macro::TokenStream::from(
            quote! { damascus::spec::HeaderValue::Literal(#lit_str.to_string()) },
        ),
        HeaderValue::Ident(ident) => proc_macro::TokenStream::from(
            quote! { damascus::spec::HeaderValue::Literal(#ident.to_string()) },
        ),
        HeaderValue::Type { name, r#type } => {
            let name = name.to_string();
            proc_macro::TokenStream::from(
                quote! { damascus::spec::HeaderValue::Type{ name: #name.to_string(), r#type: damascus::type_of!(#r#type) } },
            )
        }
        HeaderValue::Pattern {
            pattern,
            name,
            r#type,
        } => {
            let pattern = pattern.to_string();
            let name = name.to_string();
            proc_macro::TokenStream::from(
                quote! { damascus::spec::HeaderValue::Pattern{ pattern: #pattern.to_string(), name: #name.to_string(), r#type: damascus::type_of!(#r#type) } },
            )
        }
    };

    proc_macro::TokenStream::from(token_stream)
}
