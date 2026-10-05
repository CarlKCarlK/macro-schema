use proc_macro2::{Literal, TokenStream};
use quote::quote;
use syn::{
    Attribute, Error, Ident, Result, Token, Visibility, braced,
    parse::{ParseStream, Parser},
    token,
};

use crate::{
    schema::{BodySpec, Default, FieldSpec, MembersSpec, Schema, Shape, escape_cell},
    value::Value,
};

pub fn expand(schema_source: &str, macro_name: &str, input: TokenStream) -> Result<TokenStream> {
    let schema: Schema = syn::parse_str(schema_source)?;
    let declaration =
        (|stream: ParseStream| Declaration::parse_top(stream, &schema.body)).parse2(input)?;
    let mut errors = Errors::default();
    let fields = resolve_fields(
        &declaration.fields,
        &schema.body,
        &declaration.name,
        None,
        &mut errors,
    );
    let members =
        schema.body.members.as_ref().map(|members_spec| {
            resolve_members(macro_name, &declaration, members_spec, &mut errors)
        });
    errors.finish()?;

    let mut doc = instance_doc(macro_name, None, &declaration.name, &fields)?;
    let members_tokens = match &members {
        Some(members) => {
            let names: Vec<String> = members
                .iter()
                .map(|(member, _)| format!("`{}`", member.name))
                .collect();
            doc.push_str(&format!("\nMembers: {}.\n", names.join(", ")));
            let entries = members
                .iter()
                .enumerate()
                .map(|(index, (member, fields))| {
                    let member_doc =
                        instance_doc(macro_name, Some(&declaration.name), &member.name, fields)?;
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
    let generator = &schema.generator;
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
        let declaration = Self::parse(input, body)?;
        if !input.is_empty() {
            return Err(input.error("unexpected tokens after the declaration body"));
        }
        Ok(declaration)
    }

    fn parse(input: ParseStream, body: &BodySpec) -> Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let vis = input.parse()?;
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
                members.push(Self::parse(&content, &members_spec.body)?);
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
    if count < members_spec.min || count > members_spec.max {
        let span = members
            .get(members_spec.max)
            .map_or(declaration.name.span(), |member| member.name.span());
        errors.push(Error::new(
            span,
            format!(
                "`{macro_name}!` takes {} members; found {count}",
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
    tokens: TokenStream,
    display: String,
    is_default: bool,
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
    let (tokens, display, is_default) = match (given, &spec.shape) {
        (Some(Given::Leaf(value)), _) => (wrap(quote!(#value)), value.pretty()?, false),
        (Some(Given::Block(fields)), Shape::Block(block)) => {
            let inner = resolve_fields(fields, block, owner, member_index, errors);
            let tokens = fields_tokens(&inner);
            (wrap(quote!({ #tokens })), block_display(&inner), false)
        }
        (Some(Given::Block(_)), Shape::Leaf { .. }) => {
            return Err(Error::new(
                spec.name.span(),
                "expected a value, not a block",
            ));
        }
        (None, _) if spec.optional => (quote!([]), "(not set)".to_owned(), false),
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
            (quote!(#value), display, true)
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
            (quote!(#value), value.pretty()?, true)
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
        display,
        is_default,
    }))
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
fn instance_doc(
    macro_name: &str,
    group: Option<&Ident>,
    name: &Ident,
    fields: &[Resolved],
) -> Result<String> {
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
