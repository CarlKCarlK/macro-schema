use const_structures::const_structure;

struct Button {
    pin: u8,
}

const_structure! {
    BUTTON: Button { pin: 13, pin: 14 }
}

fn main() {}
