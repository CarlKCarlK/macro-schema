use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Attribute, Error, Expr, ExprLit, Ident, Lit, Meta, MetaNameValue, Path, Result, Token, braced,
    parse::{Parse, ParseStream},
};

use crate::value::{Kind, Value};

/// `define!` input: `{ATTR} [pub] NAME => SCHEMA`.
pub struct Definition {
    pub attrs: Vec<Attribute>,
    pub name: Ident,
    /// Source text of the schema, re-parsed by the generated proc macro on each use.
    pub schema_source: String,
    pub schema: Schema,
}

impl Parse for Definition {
    fn parse(input: ParseStream) -> Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        if input.peek(Token![pub]) {
            input.parse::<Token![pub]>()?;
        }
        let name = input.parse()?;
        input.parse::<Token![=>]>()?;
        let schema_tokens: TokenStream = input.parse()?;
        let schema = syn::parse2(schema_tokens.clone())?;
        Ok(Self {
            attrs,
            name,
            schema_source: schema_tokens.to_string(),
            schema,
        })
    }
}

/// `GENERATOR_PATH { FIELD_SPEC, ... }`.
pub struct Schema {
    pub generator: Path,
    pub fields: Vec<FieldSpec>,
}

impl Parse for Schema {
    fn parse(input: ParseStream) -> Result<Self> {
        let generator = input.parse()?;
        let content;
        braced!(content in input);
        let fields: Vec<FieldSpec> = content
            .parse_terminated(FieldSpec::parse, Token![,])?
            .into_iter()
            .collect();
        let mut seen = HashSet::new();
        for field in &fields {
            if !seen.insert(field.name.to_string()) {
                return Err(Error::new(
                    field.name.span(),
                    format!("duplicate schema field `{}`", field.name),
                ));
            }
        }
        Ok(Self { generator, fields })
    }
}

impl Schema {
    pub fn field(&self, name: &Ident) -> Option<&FieldSpec> {
        self.fields.iter().find(|field| field.name == *name)
    }

    pub fn expected_list(&self) -> String {
        self.fields
            .iter()
            .map(|field| format!("`{}`", field.name))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// `{/// doc} NAME: KIND [= DEFAULT]`.
pub struct FieldSpec {
    pub doc: String,
    pub name: Ident,
    pub kind: Kind,
    pub default: Option<Value>,
}

impl Parse for FieldSpec {
    fn parse(input: ParseStream) -> Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let doc = doc_text(&attrs)?;
        let name = input.parse()?;
        input.parse::<Token![:]>()?;
        let kind = Kind::from_ident(&input.parse()?)?;
        let default = if input.peek(Token![=]) {
            input.parse::<Token![=]>()?;
            Some(kind.parse_value(input, &name)?)
        } else {
            None
        };
        Ok(Self {
            doc,
            name,
            kind,
            default,
        })
    }
}

/// Joins `#[doc = "..."]` attributes into one string; rejects other attributes.
fn doc_text(attrs: &[Attribute]) -> Result<String> {
    let mut lines = Vec::new();
    for attr in attrs {
        match &attr.meta {
            Meta::NameValue(MetaNameValue {
                path,
                value:
                    Expr::Lit(ExprLit {
                        lit: Lit::Str(text),
                        ..
                    }),
                ..
            }) if path.is_ident("doc") => {
                let line = text.value();
                lines.push(line.strip_prefix(' ').unwrap_or(&line).to_owned());
            }
            _ => {
                return Err(Error::new_spanned(
                    attr,
                    "only doc comments are allowed on schema fields",
                ));
            }
        }
    }
    Ok(lines.join(" "))
}

pub fn define(input: TokenStream) -> Result<TokenStream> {
    let Definition {
        attrs,
        name,
        schema_source,
        schema,
    } = syn::parse2(input)?;
    let doc = macro_doc(&name, &schema)?;
    let macro_name = name.to_string();
    Ok(quote! {
        #(#attrs)*
        #[doc = #doc]
        #[proc_macro]
        pub fn #name(input: ::proc_macro::TokenStream) -> ::proc_macro::TokenStream {
            ::const_structures::__core::expand(#schema_source, #macro_name, input.into())
                .unwrap_or_else(|error| error.into_compile_error())
                .into()
        }
    })
}

/// Syntax block and field table appended to the macro's hand-written docs.
fn macro_doc(name: &Ident, schema: &Schema) -> Result<String> {
    let mut doc = String::from("\n\n**Syntax:**\n\n```text\n");
    doc.push_str(&format!("{name}! {{\n"));
    doc.push_str("    [<attributes>] [<visibility>] <Name> {\n");
    for field in &schema.fields {
        let default = match &field.default {
            Some(value) => format!(" // optional, default: {}", value.pretty()?),
            None => String::new(),
        };
        doc.push_str(&format!(
            "        {}: <{}>,{default}\n",
            field.name,
            field.kind.name()
        ));
    }
    doc.push_str("    }\n}\n```\n\n**Fields:**\n\n");
    doc.push_str("| Field | Kind | Default | Description |\n");
    doc.push_str("| ----- | ---- | ------- | ----------- |\n");
    for field in &schema.fields {
        let default = match &field.default {
            Some(value) => format!("`{}`", escape_cell(&value.pretty()?)),
            None => "required".to_owned(),
        };
        doc.push_str(&format!(
            "| `{}` | {} | {default} | {} |\n",
            field.name,
            field.kind.name(),
            escape_cell(&field.doc)
        ));
    }
    Ok(doc)
}

pub fn escape_cell(text: &str) -> String {
    text.replace('|', "\\|")
}
