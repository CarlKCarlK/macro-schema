//! Fixture mirroring a device library's proc-macro crate.

const_structures::define! {
    pub demo_button_watch as button_watch => ::demo::__button_watch_generate {
        /// GPIO pin for the button.
        pin: ident,
        /// Debounce interval in milliseconds.
        debounce_ms: expr = 20,
    }
}
