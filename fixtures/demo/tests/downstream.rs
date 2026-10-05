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
        pub(crate) Second { panel: { width: 12 }, pin: P1, dma: DMA7 },
    }
}

macro_rules! forward_vis {
    ($vis:vis $group:ident, $member:ident) => {
        demo::strips! {
            $vis $group {
                $vis $member { pin: P5 },
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
    assert_eq!((First::INDEX, First::PIN, First::DMA, First::PANEL), (0, "P0", "DMA0", None));
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
