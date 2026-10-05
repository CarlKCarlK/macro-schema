// Control: aliasing the generated wrapper through `crate::` must fail, which is why
// `define!` emits a bare-name alias instead.
#[doc(hidden)]
pub use const_structures::expand as __const_structures_expand;

mod widgets {
    const_structures::define! {
        pub widget => __widget_generate {
            size: expr = 1,
        }
    }
}

pub use crate::__const_structures_widget as widget_by_crate_path;

fn main() {}
