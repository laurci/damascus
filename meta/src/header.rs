use syn::parse::Parse;

pub enum HeaderValue {
    Literal(syn::LitStr),
    Ident(syn::Ident),
    Type {
        name: syn::Ident,
        r#type: syn::Type,
    },
    Pattern {
        pattern: String,
        name: syn::Ident,
        r#type: syn::Type,
    },
}

impl Parse for HeaderValue {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.peek(syn::LitStr) {
            let lit_str = input.parse::<syn::LitStr>()?;

            // peek next token if it is "use"
            if input.peek(syn::Token![use]) {
                input.parse::<syn::Token![use]>()?;

                let name = input.parse::<syn::Ident>()?;
                input.parse::<syn::Token![:]>()?;

                let r#type = input.parse::<syn::Type>()?;
                return Ok(Self::Pattern {
                    pattern: lit_str.value(),
                    name,
                    r#type,
                });
            }

            Ok(Self::Literal(lit_str))
        } else if input.peek(syn::Ident) {
            let name = input.parse::<syn::Ident>()?;
            if input.peek(syn::Token![:]) {
                input.parse::<syn::Token![:]>()?;
                let r#type = input.parse::<syn::Type>()?;
                Ok(Self::Type { name, r#type })
            } else {
                Ok(Self::Ident(name))
            }
        } else {
            Err(syn::Error::new(
                input.span(),
                "Expected a literal or an identifier",
            ))
        }
    }
}
