use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    Attribute, Error, Expr, ExprLit, Ident, Lit, LitInt, Meta, MetaNameValue, Path, Result, Token,
    braced, bracketed,
    parse::{Parse, ParseStream},
    spanned::Spanned,
    token,
};

use crate::value::{Kind, Value};

/// `define!` input: `{ATTR} [pub] NAME [as MACRO_NAME] => SCHEMA`.
///
/// `NAME` is the generated proc-macro function. `MACRO_NAME` (default `NAME`)
/// is the name users see after the library re-exports it with
/// `pub use ...::NAME as MACRO_NAME;`; docs and messages use it.
pub struct Definition {
    pub attrs: Vec<Attribute>,
    pub name: Ident,
    pub macro_name: Ident,
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
        let name: Ident = input.parse()?;
        let macro_name = if input.peek(Token![as]) {
            input.parse::<Token![as]>()?;
            input.parse()?
        } else {
            name.clone()
        };
        input.parse::<Token![=>]>()?;
        let schema_tokens: TokenStream = input.parse()?;
        let schema = syn::parse2(schema_tokens.clone())?;
        Ok(Self {
            attrs,
            name,
            macro_name,
            schema_source: schema_tokens.to_string(),
            schema,
        })
    }
}

/// `GENERATOR_PATH { BODY }`.
pub struct Schema {
    pub generator: Path,
    pub body: BodySpec,
}

impl Parse for Schema {
    fn parse(input: ParseStream) -> Result<Self> {
        let generator = input.parse()?;
        let content;
        braced!(content in input);
        let body = BodySpec::parse(&content, Context::Top)?;
        Ok(Self { generator, body })
    }
}

/// Where a body appears; decides whether members and `by_index` are allowed.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Context {
    Top,
    Member,
    /// A nested block; `in_member` says whether it sits inside a member.
    Block {
        in_member: bool,
    },
}

/// The fields (and at most one members section) of a declaration or block.
pub struct BodySpec {
    pub fields: Vec<FieldSpec>,
    pub members: Option<Box<MembersSpec>>,
}

impl BodySpec {
    fn parse(input: ParseStream, context: Context) -> Result<Self> {
        let mut fields: Vec<FieldSpec> = Vec::new();
        let mut members: Option<Box<MembersSpec>> = None;
        while !input.is_empty() {
            let attrs = input.call(Attribute::parse_outer)?;
            let FieldAttrs {
                doc,
                default_display,
            } = FieldAttrs::parse(&attrs)?;
            if is_members_start(input) {
                let keyword: Ident = input.parse()?;
                if let Some((_, span)) = default_display {
                    return Err(Error::new(span, "`default_display` belongs on a field"));
                }
                if context != Context::Top {
                    return Err(Error::new(
                        keyword.span(),
                        "members are allowed only at the top level of a schema",
                    ));
                }
                if members.is_some() {
                    return Err(Error::new(keyword.span(), "duplicate members section"));
                }
                members = Some(Box::new(MembersSpec::parse(input, doc)?));
            } else {
                let field = FieldSpec::parse(input, doc, default_display, context)?;
                if fields.iter().any(|earlier| earlier.name == field.name) {
                    return Err(Error::new(
                        field.name.span(),
                        format!("duplicate schema field `{}`", field.name),
                    ));
                }
                fields.push(field);
            }
            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }
        Ok(Self { fields, members })
    }

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

/// `members MIN..=MAX` (a field named `members` is written `members: ...`).
fn is_members_start(input: ParseStream) -> bool {
    let fork = input.fork();
    fork.parse::<Ident>()
        .is_ok_and(|ident| ident == "members" && fork.peek(LitInt))
}

/// `{/// doc} members MIN..=MAX { BODY }`.
pub struct MembersSpec {
    pub doc: String,
    pub min: usize,
    pub max: usize,
    pub body: BodySpec,
}

impl MembersSpec {
    fn parse(input: ParseStream, doc: String) -> Result<Self> {
        let min_lit: LitInt = input.parse()?;
        input.parse::<Token![..=]>()?;
        let max_lit: LitInt = input.parse()?;
        let min: usize = min_lit.base10_parse()?;
        let max: usize = max_lit.base10_parse()?;
        if min > max {
            return Err(Error::new(max_lit.span(), "member range is empty"));
        }
        let content;
        braced!(content in input);
        let body = BodySpec::parse(&content, Context::Member)?;
        Ok(Self {
            doc,
            min,
            max,
            body,
        })
    }

    pub fn count_text(&self) -> String {
        if self.min == self.max {
            format!("exactly {}", self.min)
        } else {
            format!("{} to {}", self.min, self.max)
        }
    }
}

/// `{/// doc} NAME [?] : (KIND [= DEFAULT] | { BODY })`.
pub struct FieldSpec {
    pub doc: String,
    pub name: Ident,
    /// Written `name?:`; the generator receives `[]` or `[value]`.
    pub optional: bool,
    pub shape: Shape,
    /// `#[default_display = "..."]`: how docs show the default, when its generated
    /// spelling (often a full path) is unreadable.
    pub default_display: Option<String>,
}

pub enum Shape {
    Leaf {
        kind: Kind,
        default: Option<Default>,
    },
    Block(BodySpec),
}

pub enum Default {
    Value(Value),
    /// `by_index[A, B, ...]`: the member at index `i` defaults to the `i`th value.
    ByIndex(Vec<Value>),
}

impl FieldSpec {
    fn parse(
        input: ParseStream,
        doc: String,
        default_display: Option<(String, Span)>,
        context: Context,
    ) -> Result<Self> {
        let name: Ident = input.parse()?;
        let optional = input.peek(Token![?]);
        if optional {
            input.parse::<Token![?]>()?;
        }
        input.parse::<Token![:]>()?;
        let shape = if input.peek(token::Brace) {
            let content;
            braced!(content in input);
            let in_member = matches!(
                context,
                Context::Member | Context::Block { in_member: true }
            );
            Shape::Block(BodySpec::parse(&content, Context::Block { in_member })?)
        } else {
            let kind = Kind::from_ident(&input.parse()?)?;
            let default = if input.peek(Token![=]) {
                let equals = input.parse::<Token![=]>()?;
                if optional {
                    return Err(Error::new(
                        equals.span,
                        "an optional (`?`) field cannot also have a default",
                    ));
                }
                Some(Default::parse(input, kind, &name, context)?)
            } else {
                None
            };
            Shape::Leaf { kind, default }
        };
        let default_display = match default_display {
            None => None,
            Some((text, _))
                if matches!(
                    shape,
                    Shape::Leaf {
                        default: Some(Default::Value(_)),
                        ..
                    }
                ) =>
            {
                Some(text)
            }
            Some((_, span)) => {
                return Err(Error::new(
                    span,
                    "`default_display` needs a single default value (`= ...`)",
                ));
            }
        };
        Ok(Self {
            doc,
            name,
            optional,
            shape,
            default_display,
        })
    }
}

impl Default {
    fn parse(input: ParseStream, kind: Kind, field_name: &Ident, context: Context) -> Result<Self> {
        let fork = input.fork();
        let is_by_index = fork
            .parse::<Ident>()
            .is_ok_and(|ident| ident == "by_index" && fork.peek(token::Bracket));
        if !is_by_index {
            return Ok(Self::Value(kind.parse_value(input, field_name)?));
        }
        let keyword: Ident = input.parse()?;
        if matches!(context, Context::Top | Context::Block { in_member: false }) {
            return Err(Error::new(
                keyword.span(),
                "`by_index` defaults are allowed only in member fields",
            ));
        }
        let content;
        bracketed!(content in input);
        let mut values = Vec::new();
        while !content.is_empty() {
            values.push(kind.parse_value(&content, field_name)?);
            if content.is_empty() {
                break;
            }
            content.parse::<Token![,]>()?;
        }
        Ok(Self::ByIndex(values))
    }
}

/// Attributes allowed on schema fields: doc comments and `#[default_display = "..."]`.
struct FieldAttrs {
    doc: String,
    default_display: Option<(String, Span)>,
}

impl FieldAttrs {
    fn parse(attrs: &[Attribute]) -> Result<Self> {
        let mut lines = Vec::new();
        let mut default_display = None;
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
                Meta::NameValue(MetaNameValue {
                    path,
                    value:
                        Expr::Lit(ExprLit {
                            lit: Lit::Str(text),
                            ..
                        }),
                    ..
                }) if path.is_ident("default_display") => {
                    default_display = Some((text.value(), path.span()));
                }
                _ => {
                    return Err(Error::new_spanned(
                        attr,
                        "only doc comments and `#[default_display = \"...\"]` are allowed on schema fields",
                    ));
                }
            }
        }
        Ok(Self {
            doc: lines.join(" "),
            default_display,
        })
    }
}

pub fn define(input: TokenStream) -> Result<TokenStream> {
    let Definition {
        attrs,
        name,
        macro_name,
        schema_source,
        schema,
    } = syn::parse2(input)?;
    let doc = macro_doc(&macro_name, &schema)?;
    let macro_name = macro_name.to_string();
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

/// Syntax block and field tables appended to the macro's hand-written docs.
pub(crate) fn macro_doc(macro_name: &Ident, schema: &Schema) -> Result<String> {
    let mut doc = String::from("\n\n**Syntax:**\n\n```text\n");
    doc.push_str(&format!("{macro_name}! {{\n"));
    doc.push_str("    [<attributes>] [<visibility>] <Name> {\n");
    syntax_lines(&schema.body, 2, &mut doc)?;
    doc.push_str("    }\n}\n```\n\n**Fields:**\n\n");
    field_table(&schema.body.fields, &mut doc)?;
    if let Some(members) = &schema.body.members {
        doc.push_str(&format!(
            "\n**Member fields** ({} members{}):\n\n",
            members.count_text(),
            doc_suffix(&members.doc)
        ));
        field_table(&members.body.fields, &mut doc)?;
    }
    Ok(doc)
}

fn doc_suffix(doc: &str) -> String {
    if doc.is_empty() {
        String::new()
    } else {
        format!("; {doc}")
    }
}

fn syntax_lines(body: &BodySpec, depth: usize, doc: &mut String) -> Result<()> {
    let indent = "    ".repeat(depth);
    for field in &body.fields {
        match &field.shape {
            Shape::Leaf { kind, default } => {
                let note = match (default, field.optional) {
                    (Some(default), _) => {
                        format!(
                            " // optional, default: {}",
                            default_text(default, field.default_display.as_deref(), "")?
                        )
                    }
                    (None, true) => " // optional".to_owned(),
                    (None, false) => String::new(),
                };
                doc.push_str(&format!(
                    "{indent}{}: <{}>,{note}\n",
                    field.name,
                    kind.name()
                ));
            }
            Shape::Block(block) => {
                let note = if field.optional { " // optional" } else { "" };
                doc.push_str(&format!("{indent}{}: {{{note}\n", field.name));
                syntax_lines(block, depth + 1, doc)?;
                doc.push_str(&format!("{indent}}},\n"));
            }
        }
    }
    if let Some(members) = &body.members {
        doc.push_str(&format!(
            "{indent}[<attributes>] [<visibility>] <MemberName> {{ // {} members\n",
            members.count_text()
        ));
        syntax_lines(&members.body, depth + 1, doc)?;
        doc.push_str(&format!("{indent}}},\n"));
    }
    Ok(())
}

fn field_table(fields: &[FieldSpec], doc: &mut String) -> Result<()> {
    doc.push_str("| Field | Kind | Default | Description |\n");
    doc.push_str("| ----- | ---- | ------- | ----------- |\n");
    field_rows(fields, "", doc)
}

fn field_rows(fields: &[FieldSpec], prefix: &str, doc: &mut String) -> Result<()> {
    for field in fields {
        let path = format!("{prefix}{}", field.name);
        let (kind, default) = match &field.shape {
            Shape::Leaf { kind, default } => {
                let default = match (default, field.optional) {
                    (Some(default), _) => escape_cell(&default_text(
                        default,
                        field.default_display.as_deref(),
                        "`",
                    )?),
                    (None, true) => "optional".to_owned(),
                    (None, false) => "required".to_owned(),
                };
                (kind.name(), default)
            }
            Shape::Block(_) => {
                let default = if field.optional {
                    "optional"
                } else {
                    "required"
                };
                ("block", default.to_owned())
            }
        };
        doc.push_str(&format!(
            "| `{path}` | {kind} | {default} | {} |\n",
            escape_cell(&field.doc)
        ));
        if let Shape::Block(block) = &field.shape {
            field_rows(&block.fields, &format!("{path}."), doc)?;
        }
    }
    Ok(())
}

/// `quote` wraps each value, e.g. in backticks for Markdown table cells.
fn default_text(default: &Default, display: Option<&str>, quote: &str) -> Result<String> {
    Ok(match default {
        Default::Value(value) => match display {
            Some(display) => format!("{quote}{display}{quote}"),
            None => format!("{quote}{}{quote}", value.pretty()?),
        },
        Default::ByIndex(values) => {
            let values = values
                .iter()
                .map(|value| Ok(format!("{quote}{}{quote}", value.pretty()?)))
                .collect::<Result<Vec<_>>>()?;
            format!("by member index: {}", values.join(", "))
        }
    })
}

pub fn escape_cell(text: &str) -> String {
    text.replace('|', "\\|")
}
