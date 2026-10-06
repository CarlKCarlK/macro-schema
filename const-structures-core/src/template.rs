//! The `generate { ... }` template language.
//!
//! A template is ordinary Rust tokens plus four structural constructs, all introduced
//! by `$`:
//!
//! - `$value`, `$member.field`: substitute a resolved value.
//! - `$for member in $members { ... }`: repeat once per member.
//! - `$if let Some(x) = $optional { ... } else { ... }`: branch on an optional field.
//! - `$ident(...)`, `$snake(...)`, `$upper(...)`: build an identifier.
//!
//! `$crate` passes through unchanged. Templates are parsed and type-checked against the
//! schema when `define!` runs, so a typo in a template is reported at the library's
//! `generate` block, not at some later caller.

use std::iter::Peekable;

use proc_macro2::{
    Delimiter, Group, Ident, Punct, Spacing, Span, TokenStream, TokenTree, token_stream,
};
use syn::{Error, Result};

use crate::schema::{BodySpec, Shape};

/// The sigil `define!` sees. Embedded in the generated `macro_rules!` wrapper, the
/// template's own `$` would read as `macro_rules!` variables, so the wrapper carries
/// `#` instead; `$crate` stays `$crate`, which `macro_rules!` resolves for us.
pub(crate) const AUTHOR_SIGIL: char = '$';
pub(crate) const EMBEDDED_SIGIL: char = '#';

type Tokens = Peekable<token_stream::IntoIter>;

/// Re-marks template constructs from `$` to `#` for embedding in the wrapper.
pub(crate) fn embed(tokens: TokenStream) -> TokenStream {
    let mut output = Vec::new();
    let mut tokens = tokens.into_iter().peekable();
    while let Some(token) = tokens.next() {
        match token {
            TokenTree::Punct(punct)
                if punct.as_char() == AUTHOR_SIGIL
                    && matches!(tokens.peek(), Some(TokenTree::Ident(ident)) if ident != "crate") =>
            {
                let mut sigil = Punct::new(EMBEDDED_SIGIL, Spacing::Alone);
                sigil.set_span(punct.span());
                output.push(TokenTree::Punct(sigil));
            }
            TokenTree::Group(group) => {
                let mut new_group = Group::new(group.delimiter(), embed(group.stream()));
                new_group.set_span(group.span());
                output.push(TokenTree::Group(new_group));
            }
            other => output.push(other),
        }
    }
    output.into_iter().collect()
}

pub(crate) struct Template {
    nodes: Vec<Node>,
}

enum Node {
    Token(TokenTree),
    Group(Delimiter, Span, Vec<Node>),
    Value(Path),
    For {
        var: String,
        body: Vec<Node>,
    },
    IfLet {
        var: String,
        path: Path,
        then: Vec<Node>,
        otherwise: Vec<Node>,
    },
    Ident {
        case: Case,
        span: Span,
        parts: Vec<Part>,
    },
}

#[derive(Clone, Copy)]
enum Case {
    Exact,
    Snake,
    Upper,
}

enum Part {
    Text(String),
    Value(Path),
}

/// `$root.field.field`, resolved against the schema while parsing.
struct Path {
    root: Ident,
    fields: Vec<Ident>,
}

/// What a template expression refers to.
#[derive(Clone, Copy)]
enum Ty<'a> {
    /// A substitutable value; `ident_like` if it is always one identifier (or an index).
    Leaf {
        ident_like: bool,
    },
    Optional(&'a Shape),
    Block(&'a BodySpec),
    Member(&'a BodySpec),
    Members,
}

struct Scope<'a> {
    name: String,
    ty: Ty<'a>,
}

struct Parser<'a> {
    sigil: char,
    body: &'a BodySpec,
    scopes: Vec<Scope<'a>>,
}

const RESERVED: [&str; 7] = ["name", "vis", "doc", "attrs", "members", "crate", "index"];

impl Template {
    pub(crate) fn parse(tokens: TokenStream, body: &BodySpec, sigil: char) -> Result<Self> {
        let member_fields = body.members.iter().flat_map(|members| &members.body.fields);
        for field in body.fields.iter().chain(member_fields) {
            let word = field.name.to_string();
            if RESERVED.contains(&word.as_str()) {
                return Err(Error::new(
                    field.name.span(),
                    format!(
                        "field `{word}` collides with the template's built-in `${word}`; rename the field"
                    ),
                ));
            }
        }
        let mut parser = Parser {
            sigil,
            body,
            scopes: Vec::new(),
        };
        let nodes = parser.nodes(tokens)?;
        Ok(Self { nodes })
    }

    pub(crate) fn render(&self, data: &Data) -> TokenStream {
        let mut env: Vec<(String, &Data)> = Vec::new();
        let mut output = TokenStream::new();
        render_nodes(&self.nodes, data, &mut env, &mut output);
        output
    }
}

impl<'a> Parser<'a> {
    fn nodes(&mut self, tokens: TokenStream) -> Result<Vec<Node>> {
        let mut nodes = Vec::new();
        let mut tokens = tokens.into_iter().peekable();
        while let Some(token) = tokens.next() {
            match token {
                TokenTree::Punct(punct)
                    if punct.as_char() == self.sigil
                        && matches!(tokens.peek(), Some(TokenTree::Ident(ident)) if ident != "crate") =>
                {
                    let keyword = expect_ident(&mut tokens, punct.span(), "a template construct")?;
                    nodes.push(self.construct(punct.span(), keyword, &mut tokens)?);
                }
                TokenTree::Punct(punct)
                    if punct.as_char() == AUTHOR_SIGIL && self.sigil == AUTHOR_SIGIL =>
                {
                    let is_crate =
                        matches!(tokens.peek(), Some(TokenTree::Ident(ident)) if ident == "crate");
                    if !is_crate {
                        return Err(Error::new(
                            punct.span(),
                            "`$` must start a template construct (`$value`, `$for`, `$if`, `$ident(...)`) or `$crate`",
                        ));
                    }
                    nodes.push(Node::Token(TokenTree::Punct(punct)));
                }
                TokenTree::Group(group) => {
                    let inner = self.nodes(group.stream())?;
                    nodes.push(Node::Group(group.delimiter(), group.span(), inner));
                }
                other => nodes.push(Node::Token(other)),
            }
        }
        Ok(nodes)
    }

    fn construct(&mut self, sigil_span: Span, keyword: Ident, tokens: &mut Tokens) -> Result<Node> {
        let word = keyword.to_string();
        let is_call = matches!(
            tokens.peek(),
            Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Parenthesis
        );
        match word.as_str() {
            "for" => self.for_node(&keyword, tokens),
            "if" => self.if_node(&keyword, tokens),
            "ident" | "snake" | "upper" if is_call => {
                let args = expect_group(tokens, keyword.span(), Delimiter::Parenthesis, "`(...)`")?;
                let case = match word.as_str() {
                    "ident" => Case::Exact,
                    "snake" => Case::Snake,
                    _ => Case::Upper,
                };
                Ok(Node::Ident {
                    case,
                    span: sigil_span,
                    parts: self.parts(args.stream(), args.span())?,
                })
            }
            _ => {
                let (path, ty) = self.path(keyword, tokens)?;
                match ty {
                    Ty::Leaf { .. } => Ok(Node::Value(path)),
                    Ty::Optional(_) => Err(path_error(
                        &path,
                        "is optional; read it with `$if let Some(x) = ... { ... }`",
                    )),
                    Ty::Members => Err(path_error(
                        &path,
                        "is the member list; loop over it with `$for member in $members { ... }`",
                    )),
                    Ty::Block(_) | Ty::Member(_) => Err(path_error(
                        &path,
                        "is a block, not a value; name one of its fields",
                    )),
                }
            }
        }
    }

    /// `$for VAR in $members { BODY }`.
    fn for_node(&mut self, keyword: &Ident, tokens: &mut Tokens) -> Result<Node> {
        let var = expect_ident(tokens, keyword.span(), "a loop variable after `$for`")?;
        expect_word(tokens, var.span(), "in")?;
        expect_sigil(tokens, self.sigil, var.span())?;
        let root = expect_ident(tokens, var.span(), "`$members` after `in`")?;
        let (path, ty) = self.path(root, tokens)?;
        let (Ty::Members, Some(members)) = (ty, &self.body.members) else {
            return Err(path_error(
                &path,
                "is not the member list; loop over `$members`",
            ));
        };
        let body = expect_group(tokens, var.span(), Delimiter::Brace, "`{ ... }`")?;
        self.check_new_name(&var)?;
        self.scopes.push(Scope {
            name: var.to_string(),
            ty: Ty::Member(&members.body),
        });
        let body = self.nodes(body.stream());
        self.scopes.pop();
        Ok(Node::For {
            var: var.to_string(),
            body: body?,
        })
    }

    /// `$if let Some(VAR) = $path { THEN } [else { OTHERWISE }]`.
    fn if_node(&mut self, keyword: &Ident, tokens: &mut Tokens) -> Result<Node> {
        expect_word(tokens, keyword.span(), "let")?;
        expect_word(tokens, keyword.span(), "Some")?;
        let binding = expect_group(tokens, keyword.span(), Delimiter::Parenthesis, "`Some(x)`")?;
        let var: Ident = syn::parse2(binding.stream())
            .map_err(|_| Error::new(binding.span(), "expected one variable name in `Some(...)`"))?;
        match tokens.next() {
            Some(TokenTree::Punct(punct)) if punct.as_char() == '=' => {}
            _ => return Err(Error::new(var.span(), "expected `=` after `Some(...)`")),
        }
        expect_sigil(tokens, self.sigil, var.span())?;
        let root = expect_ident(tokens, var.span(), "an optional value after `=`")?;
        let (path, ty) = self.path(root, tokens)?;
        let Ty::Optional(shape) = ty else {
            return Err(path_error(
                &path,
                "is not optional; `$if let` needs an optional field",
            ));
        };
        let then_tokens = expect_group(tokens, var.span(), Delimiter::Brace, "`{ ... }`")?;
        let otherwise_tokens = if matches!(tokens.peek(), Some(TokenTree::Ident(ident)) if ident == "else")
        {
            tokens.next();
            Some(expect_group(
                tokens,
                var.span(),
                Delimiter::Brace,
                "`{ ... }` after `else`",
            )?)
        } else {
            None
        };
        self.check_new_name(&var)?;
        self.scopes.push(Scope {
            name: var.to_string(),
            ty: shape_ty(false, shape),
        });
        let then = self.nodes(then_tokens.stream());
        self.scopes.pop();
        let otherwise = match otherwise_tokens {
            Some(group) => self.nodes(group.stream())?,
            None => Vec::new(),
        };
        Ok(Node::IfLet {
            var: var.to_string(),
            path,
            then: then?,
            otherwise,
        })
    }

    /// Comma-separated identifier parts: identifiers, integers, or `$` values.
    fn parts(&mut self, tokens: TokenStream, span: Span) -> Result<Vec<Part>> {
        let mut parts = Vec::new();
        let mut tokens = tokens.into_iter().peekable();
        while let Some(token) = tokens.next() {
            match token {
                TokenTree::Punct(punct) if punct.as_char() == self.sigil => {
                    let root = expect_ident(&mut tokens, punct.span(), "a value after `$`")?;
                    let (path, ty) = self.path(root, &mut tokens)?;
                    let Ty::Leaf { ident_like: true } = ty else {
                        return Err(path_error(
                            &path,
                            "is not an identifier; identifier parts must be `ident` fields, `$name`, or `$member.index`",
                        ));
                    };
                    parts.push(Part::Value(path));
                }
                TokenTree::Ident(ident) => parts.push(Part::Text(ident.to_string())),
                TokenTree::Literal(literal)
                    if literal
                        .to_string()
                        .bytes()
                        .all(|byte| byte.is_ascii_digit()) =>
                {
                    parts.push(Part::Text(literal.to_string()));
                }
                other => {
                    return Err(Error::new(
                        other.span(),
                        "identifier parts must be identifiers, integers, or `$` values",
                    ));
                }
            }
            match tokens.next() {
                None => break,
                Some(TokenTree::Punct(punct)) if punct.as_char() == ',' => {}
                Some(other) => {
                    return Err(Error::new(
                        other.span(),
                        "expected `,` between identifier parts",
                    ));
                }
            }
        }
        if parts.is_empty() {
            return Err(Error::new(span, "an identifier needs at least one part"));
        }
        Ok(parts)
    }

    /// Resolves `ROOT{.field}`, consuming `.field` only while the current value has fields,
    /// so `$panel.led_layout.width()` stops before `.width()`.
    fn path(&self, root: Ident, tokens: &mut Tokens) -> Result<(Path, Ty<'a>)> {
        let mut ty = self.root_ty(&root)?;
        let mut fields = Vec::new();
        while let Ty::Block(body) | Ty::Member(body) = ty {
            let is_member = matches!(ty, Ty::Member(_));
            let mut lookahead = tokens.clone();
            let (Some(TokenTree::Punct(dot)), Some(TokenTree::Ident(field))) =
                (lookahead.next(), lookahead.next())
            else {
                break;
            };
            if dot.as_char() != '.' {
                break;
            }
            tokens.next();
            tokens.next();
            ty = match field.to_string().as_str() {
                "name" | "index" if is_member => Ty::Leaf { ident_like: true },
                "vis" | "doc" | "attrs" if is_member => Ty::Leaf { ident_like: false },
                _ => match body.field(&field) {
                    Some(spec) => shape_ty(spec.optional, &spec.shape),
                    None => {
                        return Err(Error::new(
                            field.span(),
                            format!(
                                "no field `{field}` here; expected one of {}",
                                field_list(body, is_member)
                            ),
                        ));
                    }
                },
            };
            fields.push(field);
        }
        Ok((Path { root, fields }, ty))
    }

    fn root_ty(&self, root: &Ident) -> Result<Ty<'a>> {
        let word = root.to_string();
        if let Some(scope) = self.scopes.iter().rev().find(|scope| scope.name == word) {
            return Ok(scope.ty);
        }
        Ok(match word.as_str() {
            "name" => Ty::Leaf { ident_like: true },
            "vis" | "doc" | "attrs" => Ty::Leaf { ident_like: false },
            "members" if self.body.members.is_some() => Ty::Members,
            _ => match self.body.field(root) {
                Some(spec) => shape_ty(spec.optional, &spec.shape),
                None => {
                    return Err(Error::new(
                        root.span(),
                        format!(
                            "unknown template value `${word}`; expected one of {}",
                            self.top_list()
                        ),
                    ));
                }
            },
        })
    }

    fn check_new_name(&self, var: &Ident) -> Result<()> {
        let word = var.to_string();
        if RESERVED.contains(&word.as_str())
            || self.body.field(var).is_some()
            || self.scopes.iter().any(|scope| scope.name == word)
        {
            return Err(Error::new(
                var.span(),
                format!("`{word}` is already a template value; choose another name"),
            ));
        }
        Ok(())
    }

    fn top_list(&self) -> String {
        let mut names: Vec<String> = ["name", "vis", "doc", "attrs"]
            .iter()
            .map(|name| format!("`${name}`"))
            .collect();
        if self.body.members.is_some() {
            names.push("`$members`".to_owned());
        }
        names.extend(
            self.body
                .fields
                .iter()
                .map(|field| format!("`${}`", field.name)),
        );
        names.extend(self.scopes.iter().map(|scope| format!("`${}`", scope.name)));
        names.join(", ")
    }
}

fn shape_ty(optional: bool, shape: &Shape) -> Ty<'_> {
    match (optional, shape) {
        (true, shape) => Ty::Optional(shape),
        (false, Shape::Leaf { kind, .. }) => Ty::Leaf {
            ident_like: kind.name() == "ident",
        },
        (false, Shape::Block(block)) => Ty::Block(block),
    }
}

fn field_list(body: &BodySpec, is_member: bool) -> String {
    let mut names: Vec<String> = Vec::new();
    if is_member {
        names.extend(
            ["name", "vis", "doc", "attrs", "index"]
                .iter()
                .map(|name| format!("`{name}`")),
        );
    }
    names.extend(body.fields.iter().map(|field| format!("`{}`", field.name)));
    names.join(", ")
}

fn path_error(path: &Path, message: &str) -> Error {
    let text = std::iter::once(path.root.to_string())
        .chain(path.fields.iter().map(Ident::to_string))
        .collect::<Vec<_>>()
        .join(".");
    let span = path.fields.last().unwrap_or(&path.root).span();
    Error::new(span, format!("`${text}` {message}"))
}

fn expect_ident(tokens: &mut Tokens, after: Span, what: &str) -> Result<Ident> {
    match tokens.next() {
        Some(TokenTree::Ident(ident)) => Ok(ident),
        Some(other) => Err(Error::new(other.span(), format!("expected {what}"))),
        None => Err(Error::new(after, format!("expected {what}"))),
    }
}

fn expect_word(tokens: &mut Tokens, after: Span, word: &str) -> Result<()> {
    match tokens.next() {
        Some(TokenTree::Ident(ident)) if ident == word => Ok(()),
        Some(other) => Err(Error::new(other.span(), format!("expected `{word}`"))),
        None => Err(Error::new(after, format!("expected `{word}`"))),
    }
}

fn expect_sigil(tokens: &mut Tokens, sigil: char, after: Span) -> Result<()> {
    match tokens.next() {
        Some(TokenTree::Punct(punct)) if punct.as_char() == sigil => Ok(()),
        Some(other) => Err(Error::new(other.span(), "expected a `$` value")),
        None => Err(Error::new(after, "expected a `$` value")),
    }
}

fn expect_group(
    tokens: &mut Tokens,
    after: Span,
    delimiter: Delimiter,
    what: &str,
) -> Result<Group> {
    match tokens.next() {
        Some(TokenTree::Group(group)) if group.delimiter() == delimiter => Ok(group),
        Some(other) => Err(Error::new(other.span(), format!("expected {what}"))),
        None => Err(Error::new(after, format!("expected {what}"))),
    }
}

/// Resolved values a template renders from.
pub(crate) enum Data {
    Leaf(TokenStream),
    Absent,
    /// A declaration, member, or block: its named values.
    Fields(Vec<(String, Data)>),
    Members(Vec<Data>),
}

impl Data {
    /// Templates are type-checked against the same schema the data comes from, so a
    /// missing name means no output rather than an error.
    fn get(&self, name: &str) -> Option<&Data> {
        match self {
            Data::Fields(fields) => fields
                .iter()
                .find(|(field, _)| field == name)
                .map(|(_, data)| data),
            _ => None,
        }
    }
}

fn lookup<'d>(root: &'d Data, env: &[(String, &'d Data)], path: &Path) -> Option<&'d Data> {
    let name = path.root.to_string();
    let mut data = match env.iter().rev().find(|(scope, _)| *scope == name) {
        Some((_, data)) => *data,
        None => root.get(&name)?,
    };
    for field in &path.fields {
        data = data.get(&field.to_string())?;
    }
    Some(data)
}

fn render_nodes<'d>(
    nodes: &[Node],
    root: &'d Data,
    env: &mut Vec<(String, &'d Data)>,
    output: &mut TokenStream,
) {
    for node in nodes {
        match node {
            Node::Token(token) => output.extend([token.clone()]),
            Node::Group(delimiter, span, inner) => {
                let mut stream = TokenStream::new();
                render_nodes(inner, root, env, &mut stream);
                let mut group = Group::new(*delimiter, stream);
                group.set_span(*span);
                output.extend([TokenTree::Group(group)]);
            }
            Node::Value(path) => {
                if let Some(Data::Leaf(tokens)) = lookup(root, env, path) {
                    output.extend(tokens.clone());
                }
            }
            Node::For { var, body } => {
                if let Some(Data::Members(members)) = root.get("members") {
                    for member in members {
                        env.push((var.clone(), member));
                        render_nodes(body, root, env, output);
                        env.pop();
                    }
                }
            }
            Node::IfLet {
                var,
                path,
                then,
                otherwise,
            } => match lookup(root, env, path) {
                Some(Data::Absent) | None => render_nodes(otherwise, root, env, output),
                Some(data) => {
                    env.push((var.clone(), data));
                    render_nodes(then, root, env, output);
                    env.pop();
                }
            },
            Node::Ident { case, span, parts } => {
                let mut text = String::new();
                let mut ident_span = None;
                for part in parts {
                    match part {
                        Part::Text(part_text) => text.push_str(part_text),
                        Part::Value(path) => {
                            if let Some(Data::Leaf(tokens)) = lookup(root, env, path) {
                                for token in tokens.clone() {
                                    ident_span.get_or_insert(token.span());
                                    text.push_str(&token.to_string());
                                }
                            }
                        }
                    }
                }
                let text = match case {
                    Case::Exact => text,
                    Case::Snake => to_snake_case(&text),
                    Case::Upper => to_snake_case(&text).to_uppercase(),
                };
                output.extend([TokenTree::Ident(Ident::new(
                    &text,
                    ident_span.unwrap_or(*span),
                ))]);
            }
        }
    }
}

/// `Gpio0LedStrip_pin` → `gpio0_led_strip_pin`; `PIO0_BUS` → `pio0_bus`. An underscore
/// starts a word before an uppercase letter that follows a lowercase letter, or digits
/// that follow a lowercase letter, and before the last capital of an acronym
/// (`HTTPServer` → `http_server`). All-caps text keeps its words (`LED2D` → `led2d`).
pub(crate) fn to_snake_case(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut output = String::new();
    for (index, &character) in chars.iter().enumerate() {
        if character.is_uppercase() && index > 0 {
            let previous = chars[index - 1];
            let next_is_lower = chars.get(index + 1).is_some_and(|next| next.is_lowercase());
            let after_lowercase_digits = previous.is_ascii_digit()
                && chars[..index]
                    .iter()
                    .rev()
                    .find(|earlier| !earlier.is_ascii_digit())
                    .is_some_and(|earlier| earlier.is_lowercase());
            let starts_word = previous.is_lowercase()
                || after_lowercase_digits
                || (previous.is_uppercase() && next_is_lower);
            if starts_word && !output.ends_with('_') {
                output.push('_');
            }
        }
        output.extend(character.to_lowercase());
    }
    output
}
