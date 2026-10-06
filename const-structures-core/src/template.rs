//! Generates backend matchers from the declaration schema.

use std::collections::BTreeSet;

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{Error, Result};

use crate::schema::{BodySpec, FieldSpec, Shape};

/// The schema supplies every capture. Prefixing fields distinguishes ordinary fields
/// named `name`, `attrs`, etc. from declaration metadata. Nested names use their path.
pub(crate) fn matcher(body: &BodySpec) -> Result<TokenStream> {
    let mut bindings = BTreeSet::new();
    let fields = field_patterns(&body.fields, "field", &mut bindings)?;
    let members = match &body.members {
        Some(members) => {
            let fields = field_patterns(&members.body.fields, "member_field", &mut bindings)?;
            quote! {
                member_count: $member_count:literal,
                members: [$({
                    index: $member_index:literal,
                    attrs: [$(#[$member_attrs:meta])*],
                    vis: [$member_vis:vis],
                    name: $member_name:ident,
                    doc: $member_doc:literal,
                    #fields
                },)*],
            }
        }
        None => TokenStream::new(),
    };
    Ok(quote! {
        attrs: [$(#[$attrs:meta])*],
        vis: [$vis:vis],
        name: $name:ident,
        doc: $doc:literal,
        #fields
        #members
    })
}

fn field_patterns(
    specs: &[FieldSpec],
    prefix: &str,
    bindings: &mut BTreeSet<String>,
) -> Result<TokenStream> {
    let mut output = TokenStream::new();
    for spec in specs {
        let name = &spec.name;
        let binding = format_ident!("{}_{}", prefix, name, span = name.span());
        let pattern = match &spec.shape {
            Shape::Leaf { kind, .. } => {
                if !bindings.insert(binding.to_string()) {
                    return Err(Error::new(
                        name.span(),
                        format!(
                            "generated template binding `${binding}` conflicts with another field; rename this field"
                        ),
                    ));
                }
                let kind = Ident::new(kind.name(), name.span());
                quote!($#binding:#kind)
            }
            Shape::Block(block) => {
                let fields = field_patterns(&block.fields, &binding.to_string(), bindings)?;
                quote!({ #fields })
            }
        };
        let pattern = if spec.optional {
            quote!([$(#pattern)?])
        } else {
            pattern
        };
        output.extend(quote!(#name: #pattern,));
    }
    Ok(output)
}
