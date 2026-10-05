//! Fixture mirroring a device library's proc-macro crate.

const_structures::define! {
    /// Watches a button in a background task.
    pub button_watch => ::demo::__button_watch_generate {
        /// GPIO pin for the button.
        pin: ident,
        /// Debounce interval in milliseconds.
        debounce_ms: expr = 20,
    }
}
