// A typo in a template is reported at the library's `generate` block when `define!`
// runs, not later at some caller.
#[doc(hidden)]
pub use const_structures::expand as __const_structures_expand;

const_structures::define! {
    pub strips {
        members 1.. {
            pin: ident,
        },
    }
    generate {
        $for strip in $decl.members {
            const _: &str = stringify!($strip.pni);
        }
    }
}

fn main() {}
