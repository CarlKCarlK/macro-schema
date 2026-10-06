// The invocation error shown in README.md: a misspelled field name, reported at the
// caller's token. Keep the two in sync (tests/readme.rs checks the message).
#[doc(hidden)]
pub use const_structures::expand as __const_structures_expand;

const_structures::define! {
    pub setting {
        key: expr,
        value: ty,
        default: expr = ::core::default::Default::default(),
    }
    generate {
        $decl.vis struct $decl.name;
    }
}

setting! {
    pub Port { key: "server.port", value: u16, defualt: 8080 }
}

fn main() {}
