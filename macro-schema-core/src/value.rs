use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{Expr, Ident, Result, Type, parse::ParseStream};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Ident,
    Expr,
    Type,
}

impl Kind {
    pub fn from_ident(ident: &Ident) -> Result<Self> {
        match ident.to_string().as_str() {
            "ident" => Ok(Self::Ident),
            "expr" => Ok(Self::Expr),
            "ty" => Ok(Self::Type),
            _ => Err(syn::Error::new(
                ident.span(),
                "unknown field kind; expected `ident`, `expr`, or `ty`",
            )),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Ident => "ident",
            Self::Expr => "expr",
            Self::Type => "ty",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Self::Ident => "an identifier",
            Self::Expr => "an expression",
            Self::Type => "a type",
        }
    }

    pub fn parse_value(self, input: ParseStream, field_name: &Ident) -> Result<Value> {
        let parsed = match self {
            Self::Ident => input.parse().map(Value::Ident),
            Self::Expr => input.parse().map(|expr| Value::Expr(Box::new(expr))),
            Self::Type => input.parse().map(|ty| Value::Type(Box::new(ty))),
        };
        parsed.map_err(|error| {
            syn::Error::new(
                error.span(),
                format!("field `{field_name}` expects {}", self.description()),
            )
        })
    }
}

#[derive(Clone)]
pub enum Value {
    Ident(Ident),
    Expr(Box<Expr>),
    Type(Box<Type>),
}

impl Value {
    /// Human-readable source text, formatted by `prettyplease` on one line.
    pub fn pretty(&self) -> Result<String> {
        let text = match self {
            Self::Ident(ident) => return Ok(ident.to_string()),
            Self::Expr(expr) => unparse_between(quote!(const _: () = #expr;), "const _: () = ")?,
            Self::Type(ty) => unparse_between(quote!(type T = #ty;), "type T = ")?,
        };
        Ok(text.split_whitespace().collect::<Vec<_>>().join(" "))
    }
}

fn unparse_between(item: TokenStream, prefix: &str) -> Result<String> {
    let text = prettyplease::unparse(&syn::parse2::<syn::File>(item)?);
    let text = text.trim_end();
    let text = text.strip_prefix(prefix).unwrap_or(text);
    Ok(text.strip_suffix(';').unwrap_or(text).to_owned())
}

impl ToTokens for Value {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Ident(ident) => ident.to_tokens(tokens),
            Self::Expr(expr) => expr.to_tokens(tokens),
            Self::Type(ty) => ty.to_tokens(tokens),
        }
    }
}
