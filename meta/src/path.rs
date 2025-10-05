use syn::parse::Parse;

pub struct PathSegments {
    pub segments: Vec<PathSegment>,
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

pub enum PathSegment {
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
