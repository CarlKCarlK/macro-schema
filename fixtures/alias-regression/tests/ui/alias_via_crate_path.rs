// Control: aliasing the generated wrapper through `crate::` must fail, which is why
// `define!` emits a bare-name alias instead.
#[doc(hidden)]
pub use macro_schema::expand as __macro_schema_expand;

mod widgets {
    macro_schema::define! {
        pub widget => __widget_generate {
            size: expr = 1,
        }
    }
}

pub use crate::__macro_schema_wrapper_widget as widget_by_crate_path;

fn main() {}
