// The template error shown in README.md: an optional value read without `$if let`,
// reported at the template when `define!` runs, before any caller exists. Keep the two
// in sync (tests/readme.rs checks the message).
#[doc(hidden)]
pub use const_structures::expand as __const_structures_expand;

const_structures::define! {
    pub setting {
        key: expr,
        value: ty,
        env_var?: expr,
    }
    generate {
        $decl.vis struct $decl.name;
        impl $decl.name {
            pub const ENV_VAR: &'static str = $decl.env_var;
        }
    }
}

fn main() {}
