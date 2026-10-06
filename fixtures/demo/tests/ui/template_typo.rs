// A typo in a template is reported at the library's `generate` block when `define!`
// runs, not later at some caller.
#[doc(hidden)]
pub use macro_schema::expand as __macro_schema_expand;

macro_schema::define! {
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
