use demo::button::{button_watch, button_watch_generated::ButtonWatchGenerated};

button_watch! {
    #[derive(Debug, PartialEq)]
    pub(crate) Imported { debounce_ms: 50, pin: PIN_3 }
}

demo::button::button_watch! {
    ByPath { pin: PIN_4, }
}

#[test]
fn defaults_and_overrides() {
    assert_eq!(Imported::PIN, "PIN_3");
    assert_eq!(Imported::DEBOUNCE_MS, 50);
    assert_eq!(Imported, Imported);
    assert_eq!(ByPath::DEBOUNCE_MS, 20);
}

#[test]
fn internal_invocation() {
    assert_eq!(ButtonWatchGenerated::PIN, "PIN_13");
}

demo::strips! {
    pub Strips {
        bus: BUS1,
        First { pin: P0 },
        #[derive(Debug)]
        Second { panel: { width: 12 }, pin: P1, dma: DMA7 },
    }
}

macro_rules! forward_vis {
    ($vis:vis $group:ident, $member:ident) => {
        demo::strips! {
            $vis $group {
                $member { pin: P5 },
            }
        }
    };
}

forward_vis!(pub(crate) Forwarded, ForwardedMember);

#[test]
fn members() {
    assert_eq!(ForwardedMember::PIN, "P5");
    assert_eq!(Strips::BUS, "BUS1");
    assert_eq!(Strips::MEMBER_COUNT, 2);
    assert_eq!(
        (First::INDEX, First::PIN, First::DMA, First::PANEL),
        (0, "P0", "DMA0", None)
    );
    assert_eq!(
        (Second::INDEX, Second::PIN, Second::DMA, Second::PANEL),
        (1, "P1", "DMA7", Some((12, 6)))
    );
}

#[test]
fn ui() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/*.rs");
}

mod template_records {
    demo::records::records! {
        #[derive(Debug, PartialEq)]
        pub(crate) Complete {
            value: 7,
            range: { max: 20, min: 10 },
            output: i32,
            label: "complete",
            #[derive(Debug, PartialEq)]
            WithUpper { id: FIRST, limits: { upper: 30, min: 10 } },
            WithoutUpper { limits: { min: 5 }, id: SECOND },
        }
    }

    demo::records::records! {
        pub(super) Defaults {
            output: u16,
            label: "defaults",
            range: { min: 2 },
            WithoutLimits { id: THIRD },
        }
    }
}

#[test]
fn templates_render_nested_and_optional_values() {
    use template_records::{Complete, Defaults, WithUpper, WithoutLimits, WithoutUpper};

    assert_eq!(Complete, Complete);
    assert_eq!(WithUpper, WithUpper);
    assert_eq!(Complete::LABEL, "complete");
    assert_eq!(Complete::RANGE, (10, 20));
    assert_eq!(Complete::VALUE, Some(7));
    assert_eq!(Complete::MEMBER_COUNT, 2);
    assert_eq!(WithUpper::ID, "FIRST");
    assert_eq!(WithUpper::INDEX, 0);
    assert_eq!(WithUpper::CHANNEL, "CHANNEL_0");
    assert_eq!(WithUpper::LIMITS, Some((10, Some(30))));
    assert_eq!(WithoutUpper::INDEX, 1);
    assert_eq!(WithoutUpper::CHANNEL, "CHANNEL_1");
    assert_eq!(WithoutUpper::LIMITS, Some((5, None)));
    assert_eq!(Defaults::RANGE, (2_u16, 100_u16));
    assert_eq!(Defaults::VALUE, None);
    assert_eq!(WithoutLimits::LIMITS, None);
}

#[test]
fn expression_values_keep_their_precedence() {
    use demo::precedence::{Negative, Sum};

    assert_eq!(Sum::DOUBLED, 6);
    assert_eq!(Sum::NEGATED, -3);
    assert_eq!(Sum::Y_DOUBLED, 6);
    assert_eq!(Sum::FORWARDED, 6);
    assert_eq!(Sum::ZEROS, [0; 3]);
    assert_eq!(Sum::GENERIC, 4);
    assert_eq!(Sum::BRACED, 3);
    assert_eq!(Sum::LABEL, "scaled!");
    // A compound expression is substituted parenthesized, and `stringify!` shows that.
    assert_eq!(Sum::TEXT, "(1_i32 + 2)");
    assert_eq!(Sum::SQUARED, 9);
    assert_eq!(Negative::SQUARED, 16);
    assert_eq!(Negative::DOUBLED, -8);
}
