//! Fixture mirroring a device library's proc-macro crate.

const_structures::define! {
    pub demo_button_watch as button_watch => ::demo::__button_watch_generate {
        /// GPIO pin for the button.
        pin: ident,
        /// Debounce interval in milliseconds.
        debounce_ms: expr = 20,
    }
}

const_structures::define! {
    pub demo_strips as strips => ::demo::__strips_generate {
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
