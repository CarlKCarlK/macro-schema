//! A small library-owned schema and template. Run with `cargo run --example normalized`.
#[doc(hidden)]
pub use const_structures::expand as __const_structures_expand;

fn main() {
    assert_eq!(SensorChannels::ADDRESS, "ADDRESS_0");
    const {
        assert!(SensorChannels::ENABLED);
    }
    assert_eq!(SensorChannels::MEMBER_COUNT, 2);
    assert_eq!(Temperature::LIMITS, Some((-40, 125)));
    assert_eq!(Humidity::LIMITS, None);
}

const_structures::define! {
    /// A named collection rendered by a template.
    pub channels {
        /// Required device address.
        address: ident,
        /// Whether polling is enabled by default.
        enabled: expr = true,
        /// One or more named channel members.
        members 1..=2 {
            /// The channel input.
            input: ident,
            /// Optional nested range limits.
            limits?: {
                /// Inclusive lower bound.
                min: expr,
                /// Inclusive upper bound.
                max: expr,
            },
        }
    }

    generate {
        $decl.attrs
        #[doc = $decl.doc]
        $decl.vis struct $decl.name;

        impl $decl.name {
            pub const ADDRESS: &'static str = stringify!($decl.address);
            pub const ENABLED: bool = $decl.enabled;
            pub const MEMBER_COUNT: usize = 0 $for channel in $decl.members { + 1 };
        }

        $for channel in $decl.members {
            $channel.attrs
            #[doc = $channel.doc]
            $channel.vis struct $channel.name;

            impl $channel.name {
                pub const INDEX: usize = $channel.index;
                pub const INPUT: &'static str = stringify!($channel.input);
                pub const LIMITS: Option<(i32, i32)> = $if let Some(limits) = $channel.limits {
                    Some(($limits.min, $limits.max))
                } else {
                    None
                };
            }
        }
    }
}

channels! {
    #[derive(Debug)]
    pub SensorChannels {
        address: ADDRESS_0,
        Temperature { input: INPUT_1, limits: { min: -40, max: 125 } },
        Humidity { input: INPUT_2 },
    }
}
