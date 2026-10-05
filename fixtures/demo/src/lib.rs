//! Fixture mirroring a device library: a hand-written generator, a module
//! re-export of the schema-declared macro, and an internal invocation.

// Lets `::demo::...` paths emitted by demo-macros resolve inside this crate.
extern crate self as demo;

pub mod button {
    //! Button support.

    /// Watches a button in a background task.
    ///
    /// ```rust,no_run
    /// demo::button::button_watch! {
    ///     pub DocButton { pin: PIN_13 }
    /// }
    /// assert_eq!(DocButton::DEBOUNCE_MS, 20);
    /// ```
    #[doc(inline)]
    pub use demo_macros::demo_button_watch as button_watch;

    pub mod button_watch_generated {
        //! Example of what [`button_watch!`](super::button_watch) generates.

        super::button_watch! {
            pub ButtonWatchGenerated { pin: PIN_13 }
        }
    }
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
