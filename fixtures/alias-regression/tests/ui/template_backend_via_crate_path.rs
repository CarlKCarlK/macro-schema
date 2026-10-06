#![forbid(macro_expanded_macro_exports_accessed_by_absolute_paths)]

const_structures::define! {
    pub item => __item_generate { size: expr = 1 }
    generate { $vis struct $name; }
}

pub use crate::__const_structures_backend_item as forbidden_backend;

fn main() {}
