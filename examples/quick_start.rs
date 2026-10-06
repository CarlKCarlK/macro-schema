// examples/quick_start.rs: run with `cargo run --example quick_start`.

// ----- Library author: defines the `setting!` declaration macro -----

// Every crate that defines macros with `macro-schema` re-exports this once,
// at its crate root.
#[doc(hidden)]
pub use macro_schema::expand as __macro_schema_expand;

pub mod settings {
    /// A named configuration setting with a typed value.
    pub trait Setting {
        /// The setting's value type.
        type Value: core::str::FromStr;
        /// The key that identifies the setting in configuration text.
        const KEY: &'static str;
        /// The value used when the configuration doesn't mention the setting.
        fn default_value() -> Self::Value;

        /// Reads the setting from `key=value` lines, falling back to its default.
        fn read(config: &str) -> Self::Value {
            config
                .lines()
                .filter_map(|line| line.split_once('='))
                .find(|(key, _)| key.trim() == Self::KEY)
                .and_then(|(_, value)| value.trim().parse().ok())
                .unwrap_or_else(Self::default_value)
        }
    }

    macro_schema::define! {
        /// Declares a configuration setting: a type that implements [`Setting`].
        pub setting {
            /// Key that identifies the setting, such as `"server.port"`.
            key: expr,
            /// Type of the setting's value.
            value: ty,
            /// Value used when the setting is absent.
            #[default_display = "Default::default()"]
            default: expr = ::core::default::Default::default(),
        }

        generate {
            $decl.attrs
            #[doc = $decl.doc]
            $decl.vis struct $decl.name;

            impl $crate::settings::Setting for $decl.name {
                type Value = $decl.value;
                const KEY: &'static str = $decl.key;
                fn default_value() -> Self::Value {
                    $decl.default
                }
            }
        }
    }
}

// ----- Library user: declares settings and uses the generated types -----

use settings::{Setting, setting};

setting! {
    /// TCP port the server listens on.
    pub Port { key: "server.port", value: u16, default: 8080 }
}

setting! {
    pub Verbose { key: "log.verbose", value: bool }
}

fn main() {
    let config = "server.port = 3000\n";

    let port = Port::read(config);
    let verbose = Verbose::read(config);
    println!("{} = {port}", Port::KEY);
    println!("{} = {verbose}", Verbose::KEY);

    assert_eq!(port, 3000);
    assert!(!verbose); // not in `config`, so `bool::default()`
}
