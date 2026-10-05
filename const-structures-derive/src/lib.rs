use proc_macro::TokenStream;

#[proc_macro]
pub fn define(input: TokenStream) -> TokenStream {
    const_structures_core::define(input.into())
        .unwrap_or_else(|error| error.into_compile_error())
        .into()
}

#[proc_macro]
pub fn expand(input: TokenStream) -> TokenStream {
    const_structures_core::expand(input.into())
        .unwrap_or_else(|error| error.into_compile_error())
        .into()
}
