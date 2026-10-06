//! A small library-owned schema and backend. Run with `cargo run --example normalized`.
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
    /// A named collection normalized by this example's tiny backend.
    pub channels => __channels_generate {
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
}

#[doc(hidden)]
// Must be public because the exported schema macro invokes it from downstream crates.
#[macro_export]
macro_rules! __channels_generate {
    (
        attrs: [$(#[$attr:meta])*],
        vis: [$vis:vis],
        name: $name:ident,
        doc: $doc:literal,
        address: $address:ident,
        enabled: $enabled:expr,
        member_count: $member_count:literal,
        members: [$({
            index: $index:literal,
            attrs: [$(#[$member_attr:meta])*],
            vis: [$member_vis:vis],
            name: $member:ident,
            doc: $member_doc:literal,
            input: $input:ident,
            limits: [$({ min: $min:expr, max: $max:expr, })?],
        },)*],
    ) => {
        $(#[$attr])*
        #[doc = $doc]
        $vis struct $name;

        impl $name {
            pub const ADDRESS: &'static str = stringify!($address);
            pub const ENABLED: bool = $enabled;
            pub const MEMBER_COUNT: usize = $member_count;
        }

        $(
            $(#[$member_attr])*
            #[doc = $member_doc]
            $member_vis struct $member;

            impl $member {
                pub const INDEX: usize = $index;
                pub const INPUT: &'static str = stringify!($input);
                pub const LIMITS: Option<(i32, i32)> = {
                    let values = [None $(, Some(($min, $max)))?];
                    values[values.len() - 1]
                };
            }
        )*
    };
}

channels! {
    #[derive(Debug)]
    pub SensorChannels {
        address: ADDRESS_0,
        Temperature { input: INPUT_1, limits: { min: -40, max: 125 } },
        Humidity { input: INPUT_2 },
    }
}
