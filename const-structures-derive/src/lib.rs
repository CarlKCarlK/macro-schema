use proc_macro::TokenStream;

#[proc_macro]
pub fn const_structure(input: TokenStream) -> TokenStream {
    const_structures_core::const_structure(input.into())
        .unwrap_or_else(|error| error.into_compile_error())
        .into()
}
