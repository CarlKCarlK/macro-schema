use proc_macro2::{Group, Span, TokenStream, TokenTree};
use quote::{format_ident, quote};
use syn::{
    Attribute, Error, Expr, ExprLit, Ident, Lit, LitInt, Meta, MetaNameValue, Path, Result, Token,
    Visibility, braced, bracketed,
    parse::{Parse, ParseStream, Parser},
    spanned::Spanned,
    token,
};

use crate::{
    template::{AUTHOR_SIGIL, Template, embed},
    value::{Kind, Value},
};

/// `define!` input, in one of two forms:
///
/// - `{ATTR} VIS NAME { BODY } generate { TEMPLATE }`: `expand!` renders the template.
/// - `{ATTR} VIS NAME => GENERATOR_PATH { BODY }`: `expand!` calls a library
///   `macro_rules!` generator, at a path relative to the defining crate's root.
pub struct Definition {
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub name: Ident,
    /// Body tokens, embedded verbatim in the generated wrapper so `expand!` keeps their spans.
    pub body_tokens: TokenStream,
    pub body: BodySpec,
    pub output: DefinitionOutput,
}

pub enum DefinitionOutput {
    Generator(Path),
    Template(TokenStream),
}

impl Parse for Definition {
    fn parse(input: ParseStream) -> Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let vis = input.parse()?;
        let name: Ident = input.parse()?;
        let generator = if input.peek(Token![=>]) {
            input.parse::<Token![=>]>()?;
            let generator: Path = input.parse()?;
            if let Some(leading_colon) = generator.leading_colon {
                return Err(Error::new_spanned(
                    leading_colon,
                    "the generator path is relative to this crate's root; drop the leading `::`",
                ));
            }
            Some(generator)
        } else {
            None
        };
        let content;
        braced!(content in input);
        let body_tokens: TokenStream = content.parse()?;
        let body = parse_body.parse2(dollar_crate_as_crate(body_tokens.clone()))?;
        let output = match generator {
            Some(generator) => {
                if input.peek(syn::Ident)
                    && input
                        .fork()
                        .parse::<Ident>()
                        .is_ok_and(|word| word == "generate")
                {
                    return Err(input.error(
                        "a `generate { ... }` template replaces `=> generator`; use one or the other",
                    ));
                }
                if !input.is_empty() {
                    return Err(input.error("unexpected tokens after the schema"));
                }
                DefinitionOutput::Generator(generator)
            }
            None => {
                let keyword: Ident = input
                    .parse()
                    .map_err(|_| input.error("expected `generate { ... }` after the schema"))?;
                if keyword != "generate" {
                    return Err(Error::new(keyword.span(), "expected `generate { ... }`"));
                }
                let content;
                braced!(content in input);
                let template: TokenStream = content.parse()?;
                Template::parse(template.clone(), &body, AUTHOR_SIGIL)?;
                DefinitionOutput::Template(template)
            }
        };
        Ok(Self {
            attrs,
            vis,
            name,
            body_tokens,
            body,
            output,
        })
    }
}

/// `define!` receives `$crate` in schema defaults as two tokens, `$` and `crate`, which
/// syn cannot parse. For validation and docs, read it as `crate`; the wrapper embeds
/// the original tokens, where `macro_rules!` turns them into a real `$crate`.
fn dollar_crate_as_crate(tokens: TokenStream) -> TokenStream {
    let mut output = Vec::new();
    let mut tokens = tokens.into_iter().peekable();
    while let Some(token) = tokens.next() {
        match token {
            TokenTree::Punct(punct)
                if punct.as_char() == '$'
                    && matches!(tokens.peek(), Some(TokenTree::Ident(ident)) if ident == "crate") =>
                {}
            TokenTree::Group(group) => {
                let mut new_group =
                    Group::new(group.delimiter(), dollar_crate_as_crate(group.stream()));
                new_group.set_span(group.span());
                output.push(TokenTree::Group(new_group));
            }
            other => output.push(other),
        }
    }
    output.into_iter().collect()
}

/// Parses a top-level schema body (the part inside the braces).
pub fn parse_body(input: ParseStream) -> Result<BodySpec> {
    BodySpec::parse(input, Context::Top)
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

/// `{/// doc} members MIN..=MAX { BODY }` or, with no upper limit, `members MIN.. { BODY }`.
pub struct MembersSpec {
    pub doc: String,
    pub min: usize,
    pub max: Option<usize>,
    pub body: BodySpec,
}

impl MembersSpec {
    fn parse(input: ParseStream, doc: String) -> Result<Self> {
        let min_lit: LitInt = input.parse()?;
        let min: usize = min_lit.base10_parse()?;
        let max = if input.peek(Token![..=]) {
            input.parse::<Token![..=]>()?;
            let max_lit: LitInt = input.parse()?;
            let max: usize = max_lit.base10_parse()?;
            if min > max {
                return Err(Error::new(max_lit.span(), "member range is empty"));
            }
            Some(max)
        } else {
            input.parse::<Token![..]>()?;
            None
        };
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

    /// The allowed member count, with its noun: "at least 1 member", "1 to 4 members".
    pub fn count_text(&self) -> String {
        let noun = |count: usize| if count == 1 { "member" } else { "members" };
        match self.max {
            None => format!("at least {} {}", self.min, noun(self.min)),
            Some(max) if max == self.min => format!("exactly {max} {}", noun(max)),
            Some(max) => format!("{} to {max} members", self.min),
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

/// Implements `macro_schema::define!`: parses and checks a definition, then emits
/// a hidden exported `macro_rules!` wrapper plus a bare-name `use` alias.
///
/// The alias must name the wrapper without a `crate::` path: rustc rejects
/// absolute-path access to a `#[macro_export]` macro produced by macro expansion
/// (`macro_expanded_macro_exports_accessed_by_absolute_paths`), but re-exporting it
/// from textual scope gives it an ordinary path that other modules and crates can use.
pub fn define(input: TokenStream) -> Result<TokenStream> {
    let Definition {
        attrs,
        vis,
        name,
        body_tokens,
        body,
        output,
    } = syn::parse2(input)?;
    let doc = macro_doc(&name, &body)?;
    let macro_name = name.to_string();
    let wrapper = format_ident!("__macro_schema_wrapper_{}", name);
    let shared_attrs = attrs.iter().filter(|attr| !attr.path().is_ident("doc"));
    let output = match output {
        DefinitionOutput::Generator(generator) => quote!(generator: { $crate::#generator }),
        DefinitionOutput::Template(template) => {
            let template = embed(template);
            quote!(template: { #template })
        }
    };
    // The generated syntax and field tables go on the wrapper, the hand-written docs on
    // the alias. Rustdoc shows a re-export's own docs followed by the original item's
    // docs, so this crate's page gets both, and another crate that re-exports the macro
    // with its own docs keeps the generated tables.
    Ok(quote! {
        #(#shared_attrs)*
        #[doc = #doc]
        #[doc(hidden)]
        #[macro_export]
        macro_rules! #wrapper {
            ($($input:tt)*) => {
                $crate::__macro_schema_expand! {
                    macro_name: #macro_name,
                    #output,
                    schema: { #body_tokens },
                    input: { $($input)* },
                }
            };
        }

        #(#attrs)*
        #[doc(inline)]
        #vis use #wrapper as #name;
    })
}

/// Syntax block and field tables appended to the macro's hand-written docs.
pub(crate) fn macro_doc(macro_name: &Ident, body: &BodySpec) -> Result<String> {
    let mut doc = String::from("\n\n**Syntax:**\n\n```text\n");
    doc.push_str(&format!("{macro_name}! {{\n"));
    doc.push_str("    [<attributes>] [<visibility>] <Name> {\n");
    syntax_lines(body, 2, &mut doc)?;
    doc.push_str("    }\n}\n```\n\n**Fields:**\n\n");
    field_table(&body.fields, &mut doc)?;
    if let Some(members) = &body.members {
        doc.push_str(&format!(
            "\n**Member fields** ({}):\n\n",
            members.count_text()
        ));
        if !members.doc.is_empty() {
            doc.push_str(&format!("{}\n\n", members.doc));
        }
        field_table(&members.body.fields, &mut doc)?;
    }
    Ok(doc)
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
            "{indent}[<attributes>] <MemberName> {{ // {}; visibility comes from the group\n",
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
