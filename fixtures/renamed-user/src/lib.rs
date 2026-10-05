//! Uses demo's macros through a renamed dependency; `$crate` must still find demo.

use renamed_demo::button::button_watch;

button_watch! {
    pub Imported { pin: PIN_3 }
}

renamed_demo::strips! {
    pub Strips {
        First { pin: P0 },
    }
}
