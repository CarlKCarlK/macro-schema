//! Fixture mirroring a device library: schemas declared next to their generators,
//! module and crate-root re-exports, internal invocations, and `$crate` in defaults.
#![forbid(macro_expanded_macro_exports_accessed_by_absolute_paths)]

#[doc(hidden)]
pub use macro_schema::expand as __macro_schema_expand;

/// Default debounce interval, reached from a schema default through `$crate`.
pub const DEFAULT_DEBOUNCE_MS: u32 = 20;

pub mod button {
    //! Button support.

    macro_schema::define! {
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

macro_schema::define! {
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
    macro_schema::define! {
        /// Declares a typed record range and its members, rendered by a template.
        pub records {
            /// Label.
            label: expr,
            /// Type of range bounds.
            output: ty,
            /// Required nested range, including a nested default.
            range: { min: expr, max: expr = 100 },
            /// Optional initial value.
            value?: expr,
            /// Named records with optional limits.
            members 1..=2 {
                /// Identifier.
                id: ident,
                /// Per-record channel default.
                channel: ident = by_index[CHANNEL_0, CHANNEL_1],
                /// Optional limits, with an optional upper bound.
                limits?: { min: expr, upper?: expr },
            },
        }

        generate {
            $decl.attrs
            #[doc = $decl.doc]
            $decl.vis struct $decl.name;

            impl $decl.name {
                pub const LABEL: &'static str = $decl.label;
                pub const RANGE: ($decl.output, $decl.output) = ($decl.range.min, $decl.range.max);
                pub const VALUE: Option<i32> = $if let Some(initial) = $decl.value {
                    Some($initial)
                } else {
                    None
                };
                pub const MEMBER_COUNT: usize = 0 $for record in $decl.members { + 1 };
            }

            $for record in $decl.members {
                $record.attrs
                #[doc = $record.doc]
                $record.vis struct $record.name;

                impl $record.name {
                    pub const ID: &'static str = stringify!($record.id);
                    pub const CHANNEL: &'static str = stringify!($record.channel);
                    pub const INDEX: usize = $record.index;
                    pub const LIMITS: Option<(i32, Option<i32>)> = $if let Some(limits) = $record.limits {
                        Some(($limits.min, $if let Some(upper) = $limits.upper { Some($upper) } else { None }))
                    } else {
                        None
                    };
                }
            }
        }
    }
}

/// Expression values keep their own precedence when a template substitutes them, and
/// still work where Rust wants a literal-like expression.
pub mod precedence {
    /// Holds a const generic, to substitute an expression as a generic argument.
    pub struct Holder<const N: usize>;

    impl<const N: usize> Holder<N> {
        pub const N: usize = N;
    }

    macro_rules! forward_expr {
        ($value:expr) => {
            $value
        };
    }
    pub(crate) use forward_expr;

    macro_schema::define! {
        /// Substitutes expression fields into operator, array, generic, and macro positions.
        pub scaled {
            /// An integer expression.
            x: expr,
            /// An optional integer expression.
            y?: expr,
            /// A length; an operator expression, so a generic argument needs braces.
            n: expr = 2 + 1,
            /// A literal length, usable as a bare generic argument.
            len: expr = 4,
            /// A string literal.
            label: expr = "scaled",
        }

        generate {
            $decl.vis struct $decl.name;

            impl $decl.name {
                pub const DOUBLED: i32 = $decl.x * 2;
                pub const NEGATED: i32 = -$decl.x;
                pub const Y_DOUBLED: i32 = $if let Some(y) = $decl.y { $y * 2 } else { 0 };
                pub const FORWARDED: i32 = $crate::precedence::forward_expr!($decl.x) * 2;
                pub const ZEROS: [u8; $decl.n] = [0; $decl.n];
                pub const GENERIC: usize = $crate::precedence::Holder::<$decl.len>::N;
                pub const BRACED: usize = $crate::precedence::Holder::<{ $decl.n }>::N;
                pub const LABEL: &'static str = concat!($decl.label, "!");
                pub const TEXT: &'static str = stringify!($decl.x);
                pub const SQUARED: i32 = $decl.x.pow(2);
            }
        }
    }

    scaled! { pub Sum { x: 1_i32 + 2, y: 4 - 1 } }
    scaled! { pub Negative { x: -4_i32 } }
}
