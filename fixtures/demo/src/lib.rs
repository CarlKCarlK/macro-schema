//! Fixture mirroring a device library: schemas declared next to their generators,
//! module and crate-root re-exports, internal invocations, and `$crate` in defaults.
#![forbid(macro_expanded_macro_exports_accessed_by_absolute_paths)]

#[doc(hidden)]
pub use const_structures::expand as __const_structures_expand;

/// Default debounce interval, reached from a schema default through `$crate`.
pub const DEFAULT_DEBOUNCE_MS: u32 = 20;

pub mod button {
    //! Button support.

    const_structures::define! {
        /// Watches a button in a background task.
        ///
        /// ```rust,no_run
        /// demo::button::button_watch! {
        ///     pub DocButton { pin: PIN_13 }
        /// }
        /// assert_eq!(DocButton::DEBOUNCE_MS, 20);
        /// ```
        pub button_watch => __button_watch_generate {
            /// GPIO pin for the button.
            pin: ident,
            /// Debounce interval in milliseconds.
            #[default_display = "20"]
            debounce_ms: expr = $crate::DEFAULT_DEBOUNCE_MS,
        }
    }

    pub mod button_watch_generated {
        //! Example of what [`button_watch!`](super::button_watch) generates.

        super::button_watch! {
            pub ButtonWatchGenerated { pin: PIN_13 }
        }
    }
}

// Crate-root path kept for `demo::button_watch!`; documented in `button`.
#[doc(hidden)]
pub use button::button_watch;

macro_rules! internal_via_another_macro {
    () => {
        crate::button_watch! {
            pub InternalViaMacro { pin: PIN_2 }
        }
    };
}
internal_via_another_macro!();

const_structures::define! {
    /// Groups strips that share a bus.
    pub strips => __strips_generate {
        /// Shared bus.
        bus: ident = BUS0,
        /// One strip per member.
        members 1..=2 {
            /// Data pin.
            pin: ident,
            /// Channel.
            dma: ident = by_index[DMA0, DMA1],
            /// Optional panel geometry.
            panel?: {
                /// Width in pixels.
                width: expr,
                /// Font.
                font: expr = 6,
            },
        },
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! __strips_generate {
    (
        attrs: [$(#[$attr:meta])*],
        vis: [$vis:vis],
        name: $name:ident,
        doc: $doc:literal,
        bus: $bus:ident,
        member_count: $member_count:literal,
        members: [$({
            index: $index:literal,
            attrs: [$(#[$member_attr:meta])*],
            vis: [$member_vis:vis],
            name: $member:ident,
            doc: $member_doc:literal,
            pin: $pin:ident,
            dma: $dma:ident,
            panel: [$({ width: $width:expr, font: $font:expr, })?],
        },)*],
    ) => {
        $(#[$attr])*
        #[doc = $doc]
        $vis struct $name;

        impl $name {
            pub const BUS: &'static str = stringify!($bus);
            pub const MEMBER_COUNT: usize = $member_count;
        }

        $(
            $(#[$member_attr])*
            #[doc = $member_doc]
            $member_vis struct $member;

            impl $member {
                pub const INDEX: usize = $index;
                pub const PIN: &'static str = stringify!($pin);
                pub const DMA: &'static str = stringify!($dma);
                pub const PANEL: Option<(usize, usize)> = {
                    let options = [None $(, Some(($width, $font)))?];
                    options[options.len() - 1]
                };
            }
        )*
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __button_watch_generate {
    (
        attrs: [$(#[$attr:meta])*],
        vis: [$vis:vis],
        name: $name:ident,
        doc: $doc:literal,
        pin: $pin:ident,
        debounce_ms: $debounce_ms:expr,
    ) => {
        $(#[$attr])*
        #[doc = $doc]
        $vis struct $name;

        impl $name {
            pub const PIN: &'static str = stringify!($pin);
            pub const DEBOUNCE_MS: u32 = $debounce_ms;
        }
    };
}

/// A domain-neutral template with typed values and nested optional members.
pub mod records {
    const_structures::define! {
        /// Declares a typed record range and its members without a backend matcher.
        pub records => records::__records_generate {
            /// Label; intentionally shares a name with declaration metadata.
            name: expr,
            /// Type of range bounds.
            output: ty,
            /// Required nested range, including a nested default.
            range: { min: expr, max: expr = 100 },
            /// Optional initial value.
            value?: expr,
            /// Named records with optional limits.
            members 1..=2 {
                /// Identifier; intentionally shares a name with member metadata.
                name: ident,
                /// Per-record channel default.
                channel: ident = by_index[CHANNEL_0, CHANNEL_1],
                /// Optional limits, with an optional upper bound.
                limits?: { min: expr, upper?: expr },
            },
        }

        generate {
            $(#[$attrs])*
            #[doc = $doc]
            $vis struct $name;

            impl $name {
                pub const LABEL: &'static str = $field_name;
                pub const RANGE: ($field_output, $field_output) = ($field_range_min, $field_range_max);
                pub const VALUE: Option<i32> = {
                    let values = [None $(, Some($field_value))?];
                    values[values.len() - 1]
                };
                pub const MEMBER_COUNT: usize = $member_count;
            }

            $(
                $(#[$member_attrs])*
                #[doc = $member_doc]
                $member_vis struct $member_name;

                impl $member_name {
                    pub const NAME: &'static str = stringify!($member_field_name);
                    pub const CHANNEL: &'static str = stringify!($member_field_channel);
                    pub const INDEX: usize = $member_index;
                    pub const LIMITS: Option<(i32, Option<i32>)> = {
                        let values = [None $(, Some(($member_field_limits_min, {
                            let upper = [None $(, Some($member_field_limits_upper))?];
                            upper[upper.len() - 1]
                        })))?];
                        values[values.len() - 1]
                    };
                }
            )*
        }
    }
}
