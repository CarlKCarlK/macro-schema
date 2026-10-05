//! `proc_macro2` implementation of `const-structures`.
//!
//! Everything here is ordinary Rust, so it can be unit-tested and stepped
//! through in a debugger without going through the compiler's macro expander.

use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Expr, Ident, Path, Token, Visibility, braced,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};

pub use syn::Error;
pub type Result<T> = syn::Result<T>;

// todo Smoke-test grammar only; the real grammar is still an open question in the spec.
pub fn const_structure(input: TokenStream) -> Result<TokenStream> {
    let ConstStructure {
        vis,
        name,
        ty,
        fields,
    } = syn::parse2(input)?;
    check_unique(&fields)?;
    let field_names = fields.iter().map(|field| &field.name);
    let field_values = fields.iter().map(|field| &field.value);
    Ok(quote! {
        #vis const #name: #ty = #ty { #(#field_names: #field_values),* };
    })
}

struct ConstStructure {
    vis: Visibility,
    name: Ident,
    ty: Path,
    fields: Punctuated<Field, Token![,]>,
}

impl Parse for ConstStructure {
    fn parse(input: ParseStream) -> Result<Self> {
        let vis = input.parse()?;
        let name = input.parse()?;
        input.parse::<Token![:]>()?;
        let ty = input.parse()?;
        let content;
        braced!(content in input);
        let fields = content.parse_terminated(Field::parse, Token![,])?;
        Ok(Self {
            vis,
            name,
            ty,
            fields,
        })
    }
}

struct Field {
    name: Ident,
    value: Expr,
}

impl Parse for Field {
    fn parse(input: ParseStream) -> Result<Self> {
        let name = input.parse()?;
        input.parse::<Token![:]>()?;
        let value = input.parse()?;
        Ok(Self { name, value })
    }
}

fn check_unique(fields: &Punctuated<Field, Token![,]>) -> Result<()> {
    let mut seen = HashSet::new();
    let mut errors: Option<Error> = None;
    for field in fields {
        if !seen.insert(field.name.to_string()) {
            let error = Error::new(
                field.name.span(),
                format!("duplicate field `{}`", field.name),
            );
            match &mut errors {
                Some(errors) => errors.combine(error),
                None => errors = Some(error),
            }
        }
    }
    errors.map_or(Ok(()), Err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn pretty(tokens: TokenStream) -> Result<String> {
        Ok(prettyplease::unparse(&syn::parse2(tokens)?))
    }

    #[test]
    fn expands_named_fields() -> Result<()> {
        let input = quote! {
            pub BUTTON: Button { debounce_ms: 20, pin: 13 }
        };
        let expected = quote! {
            pub const BUTTON: Button = Button { debounce_ms: 20, pin: 13 };
        };
        assert_eq!(pretty(expected)?, pretty(const_structure(input)?)?);
        Ok(())
    }

    #[test]
    fn rejects_duplicate_field() {
        let input = quote! {
            BUTTON: Button { pin: 13, pin: 14 }
        };
        let message = const_structure(input).err().map(|error| error.to_string());
        assert_eq!(message.as_deref(), Some("duplicate field `pin`"));
    }
}
