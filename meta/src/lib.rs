use quote::quote;
use syn::parse::Parse;

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

struct PathSegments {
    segments: Vec<PathSegment>,
}

impl Parse for PathSegments {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut segments = Vec::new();
        while !input.is_empty() {
            if input.peek(syn::token::Comma) {
                input.parse::<syn::Token!(,)>()?;
            }

            segments.push(input.parse()?);
        }

        Ok(Self { segments })
    }
}

enum PathSegment {
    Literal(syn::LitStr),
    Type { name: syn::Ident, r#type: syn::Type },
}

impl Parse for PathSegment {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.peek(syn::Ident) {
            let name = input.parse::<syn::Ident>()?;
            input.parse::<syn::Token!(:)>()?;
            let r#type = input.parse::<syn::Type>()?;
            Ok(Self::Type { name, r#type })
        } else {
            Ok(Self::Literal(input.parse::<syn::LitStr>()?))
        }
    }
}
