use proc_macro2::{Delimiter, Group, Literal, TokenStream, TokenTree};
use quote::{ToTokens, quote};
use syn::{
    Attribute, Error, Expr, Ident, LitStr, Result, Token, Visibility, braced,
    parse::{Parse, ParseStream, Parser},
    token,
};

use crate::{
    schema::{BodySpec, Default, FieldSpec, MembersSpec, Shape, escape_cell, parse_body},
    template::{Data, EMBEDDED_SIGIL, Template},
    value::Value,
};

/// `expand!` input, produced by the wrapper that `define!` generates:
/// `macro_name: "NAME", generator: { PATH }, schema: { BODY }, input: { TOKENS },`.
struct ExpandInput {
    macro_name: String,
    output: Output,
    body: BodySpec,
    input: TokenStream,
}

/// Where the validated declaration goes: a library `macro_rules!` generator, or a
/// `generate { ... }` template rendered here.
pub(crate) enum Output {
    Generator(TokenStream),
    Template(TokenStream),
}

impl Parse for ExpandInput {
    fn parse(input: ParseStream) -> Result<Self> {
        expect_key(input, "macro_name")?;
        let macro_name = input.parse::<LitStr>()?.value();
        input.parse::<Token![,]>()?;
        let key: Ident = input.parse()?;
        input.parse::<Token![:]>()?;
        let tokens = braced_tokens(input)?;
        let output = match key.to_string().as_str() {
            "generator" => Output::Generator(tokens),
            "template" => Output::Template(tokens),
            _ => return Err(Error::new(key.span(), "expected `generator` or `template`")),
        };
        input.parse::<Token![,]>()?;
        expect_key(input, "schema")?;
        let content;
        braced!(content in input);
        let body = parse_body(&content)?;
        input.parse::<Token![,]>()?;
        expect_key(input, "input")?;
        let user_input = braced_tokens(input)?;
        input.parse::<Option<Token![,]>>()?;
        Ok(Self {
            macro_name,
            output,
            body,
            input: user_input,
        })
    }
}

fn expect_key(input: ParseStream, key: &str) -> Result<()> {
    let ident: Ident = input.parse()?;
    if ident != key {
        return Err(Error::new(ident.span(), format!("expected `{key}`")));
    }
    input.parse::<Token![:]>()?;
    Ok(())
}

fn braced_tokens(input: ParseStream) -> Result<TokenStream> {
    let content;
    braced!(content in input);
    content.parse()
}

pub fn expand(input: TokenStream) -> Result<TokenStream> {
    let ExpandInput {
        macro_name,
        output,
        body,
        input,
    } = syn::parse2(input)?;
    expand_parts(&macro_name, &output, &body, input)
}

pub(crate) fn expand_parts(
    macro_name: &str,
    output: &Output,
    body: &BodySpec,
    input: TokenStream,
) -> Result<TokenStream> {
    let declaration = (|stream: ParseStream| Declaration::parse_top(stream, body)).parse2(input)?;
    let mut errors = Errors::default();
    let fields = resolve_fields(
        &declaration.fields,
        body,
        &declaration.name,
        None,
        &mut errors,
    );
    let members = body
        .members
        .as_ref()
        .map(|members_spec| resolve_members(macro_name, &declaration, members_spec, &mut errors));
    errors.finish()?;

    let mut doc = instance_doc(macro_name, None, &declaration, &fields)?;
    if let (Some(members), false) = (&members, doc.is_empty()) {
        let names: Vec<String> = members
            .iter()
            .map(|(member, _)| format!("`{}`", member.name))
            .collect();
        doc.push_str(&format!("\nMembers: {}.\n", names.join(", ")));
    }
    let generator = match output {
        Output::Generator(generator) => generator,
        Output::Template(template) => {
            let template = Template::parse(template.clone(), body, EMBEDDED_SIGIL)?;
            let member_data = match &members {
                Some(members) => Some(
                    members
                        .iter()
                        .enumerate()
                        .map(|(index, (member, fields))| {
                            let member_doc =
                                instance_doc(macro_name, Some(&declaration.name), member, fields)?;
                            let mut data = header_data(member, &member_doc);
                            data.push((
                                "index".to_owned(),
                                Data::Leaf(Literal::usize_unsuffixed(index).into_token_stream()),
                            ));
                            data.extend(fields_data(fields));
                            Ok(Data::Fields(data))
                        })
                        .collect::<Result<Vec<_>>>()?,
                ),
                None => None,
            };
            let mut data = header_data(&declaration, &doc);
            data.extend(fields_data(&fields));
            if let Some(member_data) = member_data {
                data.push(("members".to_owned(), Data::Members(member_data)));
            }
            return Ok(template.render(&Data::Fields(data)));
        }
    };
    let members_tokens = match &members {
        Some(members) => {
            let entries = members
                .iter()
                .enumerate()
                .map(|(index, (member, fields))| {
                    let member_doc =
                        instance_doc(macro_name, Some(&declaration.name), member, fields)?;
                    let index = Literal::usize_unsuffixed(index);
                    let header = header_tokens(member, &member_doc);
                    let field_tokens = fields_tokens(fields);
                    Ok(quote!({ index: #index, #header #field_tokens }))
                })
                .collect::<Result<Vec<_>>>()?;
            let member_count = Literal::usize_unsuffixed(members.len());
            quote! {
                member_count: #member_count,
                members: [#(#entries,)*],
            }
        }
        None => TokenStream::new(),
    };
    let header = header_tokens(&declaration, &doc);
    let field_tokens = fields_tokens(&fields);
    Ok(quote! {
        #generator! {
            #header
            #field_tokens
            #members_tokens
        }
    })
}

/// `{ATTR} VIS NAME { ITEM, ... }` where an item is a field or a member.
struct Declaration {
    attrs: Vec<Attribute>,
    vis: Visibility,
    name: Ident,
    fields: Vec<(Ident, Given)>,
    members: Vec<Declaration>,
}

enum Given {
    Leaf(Value),
    Block(Vec<(Ident, Given)>),
}

impl Declaration {
    fn parse_top(input: ParseStream, body: &BodySpec) -> Result<Self> {
        let declaration = Self::parse(input, body, None)?;
        if !input.is_empty() {
            return Err(input.error("unexpected tokens after the declaration body"));
        }
        Ok(declaration)
    }

    /// `group_vis` is `Some` for a member, which takes its group's visibility.
    fn parse(input: ParseStream, body: &BodySpec, group_vis: Option<&Visibility>) -> Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let written_vis: Visibility = input.parse()?;
        let vis = match group_vis {
            None => written_vis,
            Some(_) if !matches!(written_vis, Visibility::Inherited) => {
                return Err(Error::new_spanned(
                    written_vis,
                    "members take their group's visibility; remove this visibility",
                ));
            }
            Some(group_vis) => group_vis.clone(),
        };
        let name: Ident = input.parse()?;
        if input.peek(Token![:]) && input.peek2(token::Brace) {
            return Err(input.error(format!(
                "remove the `:` after `{name}`; declarations are written `{name} {{ ... }}`"
            )));
        }
        let content;
        braced!(content in input);
        let mut fields = Vec::new();
        let mut members = Vec::new();
        while !content.is_empty() {
            if is_member_start(&content) {
                let Some(members_spec) = &body.members else {
                    return Err(content.error(format!(
                        "`{name}` takes fields only; expected one of {}",
                        body.expected_list()
                    )));
                };
                members.push(Self::parse(&content, &members_spec.body, Some(&vis))?);
            } else {
                fields.push(parse_field(&content, body)?);
            }
            if content.is_empty() {
                break;
            }
            content.parse::<Token![,]>()?;
        }
        Ok(Self {
            attrs,
            vis,
            name,
            fields,
            members,
        })
    }
}

/// A member is `{ATTR} VIS Name {`; a field is `name:`. Parsing ahead (rather than
/// peeking for `pub`) also handles a visibility forwarded as a `macro_rules!` `$vis`.
fn is_member_start(input: ParseStream) -> bool {
    if input.peek(Token![#]) {
        return true;
    }
    let fork = input.fork();
    fork.parse::<Visibility>().is_ok() && fork.parse::<Ident>().is_ok() && fork.peek(token::Brace)
}

fn parse_field(input: ParseStream, body: &BodySpec) -> Result<(Ident, Given)> {
    let field_name: Ident = input.parse()?;
    if body.field(&field_name).is_none()
        && body.members.is_some()
        && input.peek(Token![:])
        && input.peek2(token::Brace)
    {
        return Err(Error::new(
            field_name.span(),
            format!(
                "remove the `:` after `{field_name}`; declarations are written `{field_name} {{ ... }}`"
            ),
        ));
    }
    let spec = body.field(&field_name).ok_or_else(|| {
        let members_hint = if body.members.is_some() {
            " (or a member `Name { ... }`)"
        } else {
            ""
        };
        Error::new(
            field_name.span(),
            format!(
                "unknown field `{field_name}`; expected one of {}{members_hint}",
                body.expected_list()
            ),
        )
    })?;
    input.parse::<Token![:]>()?;
    let given = match &spec.shape {
        Shape::Leaf { kind, .. } => Given::Leaf(kind.parse_value(input, &field_name)?),
        Shape::Block(block) => {
            if !input.peek(token::Brace) {
                return Err(input.error(format!("field `{field_name}` expects `{{ ... }}`")));
            }
            let content;
            braced!(content in input);
            let mut fields = Vec::new();
            while !content.is_empty() {
                fields.push(parse_field(&content, block)?);
                if content.is_empty() {
                    break;
                }
                content.parse::<Token![,]>()?;
            }
            Given::Block(fields)
        }
    };
    Ok((field_name, given))
}

#[derive(Default)]
struct Errors(Option<Error>);

impl Errors {
    fn push(&mut self, error: Error) {
        match &mut self.0 {
            Some(errors) => errors.combine(error),
            None => self.0 = Some(error),
        }
    }

    fn finish(self) -> Result<()> {
        self.0.map_or(Ok(()), Err)
    }
}

fn resolve_members<'a>(
    macro_name: &str,
    declaration: &'a Declaration,
    members_spec: &MembersSpec,
    errors: &mut Errors,
) -> Vec<(&'a Declaration, Vec<Resolved>)> {
    let members = &declaration.members;
    let count = members.len();
    let too_many = members_spec.max.is_some_and(|max| count > max);
    if count < members_spec.min || too_many {
        let span = members_spec
            .max
            .and_then(|max| members.get(max))
            .map_or(declaration.name.span(), |member| member.name.span());
        errors.push(Error::new(
            span,
            format!(
                "`{macro_name}!` takes {}; found {count}",
                members_spec.count_text()
            ),
        ));
    }
    for (index, member) in members.iter().enumerate() {
        if members[..index]
            .iter()
            .any(|earlier| earlier.name == member.name)
        {
            errors.push(Error::new(
                member.name.span(),
                format!("duplicate member `{}`", member.name),
            ));
        }
    }
    members
        .iter()
        .enumerate()
        .map(|(index, member)| {
            let fields = resolve_fields(
                &member.fields,
                &members_spec.body,
                &member.name,
                Some(index),
                errors,
            );
            (member, fields)
        })
        .collect()
}

/// One schema field, resolved: the tokens the generator receives and how docs show it.
struct Resolved {
    name: Ident,
    /// What a `generator` macro receives: optional values wrapped in `[...]`.
    tokens: TokenStream,
    /// What a `generate` template reads.
    value: ResolvedValue,
    display: String,
    is_default: bool,
}

enum ResolvedValue {
    Leaf(TokenStream),
    Block(Vec<Resolved>),
    Absent,
}

fn fields_data(fields: &[Resolved]) -> Vec<(String, Data)> {
    fields
        .iter()
        .map(|field| {
            let data = match &field.value {
                ResolvedValue::Leaf(tokens) => Data::Leaf(tokens.clone()),
                ResolvedValue::Block(inner) => Data::Fields(fields_data(inner)),
                ResolvedValue::Absent => Data::Absent,
            };
            (field.name.to_string(), data)
        })
        .collect()
}

/// Every schema field in schema order, with defaults filled in; errors are collected.
fn resolve_fields(
    given: &[(Ident, Given)],
    body: &BodySpec,
    owner: &Ident,
    member_index: Option<usize>,
    errors: &mut Errors,
) -> Vec<Resolved> {
    for (index, (name, _)) in given.iter().enumerate() {
        if given[..index].iter().any(|(earlier, _)| earlier == name) {
            errors.push(Error::new(name.span(), format!("duplicate field `{name}`")));
        }
    }
    let mut resolved = Vec::new();
    for spec in &body.fields {
        let value = given
            .iter()
            .find(|(name, _)| *name == spec.name)
            .map(|(_, given)| given);
        match resolve_field(spec, value, owner, member_index, errors) {
            Ok(Some(field)) => resolved.push(field),
            Ok(None) => {}
            Err(error) => errors.push(error),
        }
    }
    resolved
}

/// `Ok(None)` means a missing required field, already recorded in `errors`.
fn resolve_field(
    spec: &FieldSpec,
    given: Option<&Given>,
    owner: &Ident,
    member_index: Option<usize>,
    errors: &mut Errors,
) -> Result<Option<Resolved>> {
    let wrap = |tokens: TokenStream| {
        if spec.optional {
            quote!([#tokens])
        } else {
            tokens
        }
    };
    let (tokens, value, display, is_default) = match (given, &spec.shape) {
        (Some(Given::Leaf(value)), _) => (
            wrap(quote!(#value)),
            ResolvedValue::Leaf(template_tokens(value)),
            value.pretty()?,
            false,
        ),
        (Some(Given::Block(fields)), Shape::Block(block)) => {
            let inner = resolve_fields(fields, block, owner, member_index, errors);
            let tokens = fields_tokens(&inner);
            let display = block_display(&inner);
            (
                wrap(quote!({ #tokens })),
                ResolvedValue::Block(inner),
                display,
                false,
            )
        }
        (Some(Given::Block(_)), Shape::Leaf { .. }) => {
            return Err(Error::new(
                spec.name.span(),
                "expected a value, not a block",
            ));
        }
        (None, _) if spec.optional => (
            quote!([]),
            ResolvedValue::Absent,
            "(not set)".to_owned(),
            false,
        ),
        (
            None,
            Shape::Leaf {
                default: Some(Default::Value(value)),
                ..
            },
        ) => {
            let display = match &spec.default_display {
                Some(display) => display.clone(),
                None => value.pretty()?,
            };
            (
                quote!(#value),
                ResolvedValue::Leaf(template_tokens(value)),
                display,
                true,
            )
        }
        (
            None,
            Shape::Leaf {
                default: Some(Default::ByIndex(values)),
                ..
            },
        ) => {
            let index = member_index.unwrap_or(0);
            let Some(value) = values.get(index) else {
                return Err(Error::new(
                    owner.span(),
                    format!(
                        "field `{}` has no default for member index {index}; give it explicitly",
                        spec.name
                    ),
                ));
            };
            (
                quote!(#value),
                ResolvedValue::Leaf(template_tokens(value)),
                value.pretty()?,
                true,
            )
        }
        (None, _) => {
            errors.push(Error::new(
                owner.span(),
                format!("missing required field `{}`", spec.name),
            ));
            return Ok(None);
        }
    };
    Ok(Some(Resolved {
        name: spec.name.clone(),
        tokens,
        value,
        display,
        is_default,
    }))
}

/// A value as a template substitutes it. A compound expression is parenthesized so
/// it keeps its own precedence: with `x: 1 + 2`, `$decl.x * 2` renders `(1 + 2) * 2`,
/// and `$decl.x.pow(2)` renders `(1 + 2).pow(2)`.
///
/// `macro_rules!` gets this from the invisible group around an `$x:expr` capture, but
/// rustc flattens invisible groups that a proc macro emits, so parentheses are the
/// only reliable boundary. Self-delimiting expressions (literals, paths, calls,
/// blocks, ...) stay bare, so they still work where Rust wants exactly that form:
/// `concat!($decl.label)`, `include_bytes!($decl.file)`, `Holder::<$decl.len>`.
fn template_tokens(value: &Value) -> TokenStream {
    match value {
        Value::Expr(expr) if !is_self_delimiting(expr) => {
            TokenTree::Group(Group::new(Delimiter::Parenthesis, expr.to_token_stream())).into()
        }
        _ => value.to_token_stream(),
    }
}

/// Whether an expression already reads as one operand in any position: an atom, a
/// delimited form, or a postfix chain on one.
fn is_self_delimiting(expr: &Expr) -> bool {
    match expr {
        Expr::Lit(_)
        | Expr::Path(_)
        | Expr::Paren(_)
        | Expr::Tuple(_)
        | Expr::Array(_)
        | Expr::Repeat(_)
        | Expr::Struct(_)
        | Expr::Macro(_)
        | Expr::Block(_)
        | Expr::Const(_)
        | Expr::Unsafe(_) => true,
        Expr::Call(call) => is_self_delimiting(&call.func),
        Expr::MethodCall(call) => is_self_delimiting(&call.receiver),
        Expr::Field(field) => is_self_delimiting(&field.base),
        Expr::Index(index) => is_self_delimiting(&index.expr),
        Expr::Try(try_expr) => is_self_delimiting(&try_expr.expr),
        Expr::Await(await_expr) => is_self_delimiting(&await_expr.base),
        _ => false,
    }
}

fn block_display(fields: &[Resolved]) -> String {
    let inner = fields
        .iter()
        .map(|field| format!("{}: {}", field.name, field.display))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{{ {inner} }}")
}

fn fields_tokens(fields: &[Resolved]) -> TokenStream {
    let names = fields.iter().map(|field| &field.name);
    let values = fields.iter().map(|field| &field.tokens);
    quote!(#(#names: #values,)*)
}

/// The declaration's own values, as a template reads them. As in the generator form, an
/// inherited visibility renders as `pub(self)`, which means the same and still matches
/// `$vis:vis` in any `macro_rules!` helper the template calls.
fn header_data(declaration: &Declaration, doc: &str) -> Vec<(String, Data)> {
    let Declaration {
        attrs, vis, name, ..
    } = declaration;
    let vis = match vis {
        Visibility::Inherited => quote!(pub(self)),
        vis => vis.to_token_stream(),
    };
    vec![
        ("name".to_owned(), Data::Leaf(name.to_token_stream())),
        ("vis".to_owned(), Data::Leaf(vis)),
        ("doc".to_owned(), Data::Leaf(quote!(#doc))),
        ("attrs".to_owned(), Data::Leaf(quote!(#(#attrs)*))),
    ]
}

fn header_tokens(declaration: &Declaration, doc: &str) -> TokenStream {
    let Declaration {
        attrs, vis, name, ..
    } = declaration;
    // `$vis:vis` cannot match an empty visibility at the end of `[...]`.
    let vis = match vis {
        Visibility::Inherited => quote!(pub(self)),
        vis => quote!(#vis),
    };
    quote! {
        attrs: [#(#attrs)*],
        vis: [#vis],
        name: #name,
        doc: #doc,
    }
}

/// Rustdoc for a generated item: the configuration this invocation used.
/// Empty when the declaration already has a written `#[doc = "..."]`, which replaces it.
fn instance_doc(
    macro_name: &str,
    group: Option<&Ident>,
    declaration: &Declaration,
    fields: &[Resolved],
) -> Result<String> {
    let has_written_doc = declaration
        .attrs
        .iter()
        .any(|attr| attr.path().is_ident("doc") && matches!(attr.meta, syn::Meta::NameValue(_)));
    if has_written_doc {
        return Ok(String::new());
    }
    let name = &declaration.name;
    let mut doc = match group {
        Some(group) => format!("Member `{name}` of `{group}`, generated by `{macro_name}!`.\n\n"),
        None => format!("Generated by `{macro_name}!`.\n\n"),
    };
    if fields.is_empty() {
        return Ok(doc);
    }
    doc.push_str("| Field | Value |\n| ----- | ----- |\n");
    for field in fields {
        let marker = if field.is_default { " (default)" } else { "" };
        doc.push_str(&format!(
            "| `{}` | `{}`{marker} |\n",
            field.name,
            escape_cell(&field.display)
        ));
    }
    Ok(doc)
}
