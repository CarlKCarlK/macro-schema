# Device Envoy Macro Survey

<!-- TODO0 consider deleting this survey once Device Envoy has been converted. -->

Survey of every user-facing `macro_rules!` DSL in Device Envoy
(`~/programs/mcu/device-envoy`, `main` at `4c1ee16f`). These are requirements
and test cases, not a compatibility target.

Sources: each macro's `**Syntax:**` doc block, checked against its matcher
arms. Where docs and matchers disagree, the matcher wins and the disagreement
is listed under [Inconsistencies](#inconsistencies).

## Notation

EBNF. `[x]` optional, `{x}` zero or more, `x | y` choice, `"x"` literal token.
"Any order" means the matcher accepts fields in any order (tt-muncher);
"fixed order" means fields must appear exactly as written.

Terminals:

| Terminal | Meaning                                                                                                                                            |
| -------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| `VIS`    | Rust visibility, possibly empty                                                                                                                    |
| `ATTR`   | `#[...]` outer attribute                                                                                                                           |
| `NAME`   | identifier naming a generated item                                                                                                                 |
| `PERIPH` | identifier naming a HAL peripheral, e.g. `PIN_3`, `PIO0`, `DMA_CH0`. It is pasted into a type path (`embassy_rp::peripherals::PIN_3`), not evaluated |
| `EXPR`   | Rust expression, usually const-evaluated                                                                                                           |
| `TYPE`   | Rust type                                                                                                                                          |

Shared productions:

```ebnf
Field<K, V>    = K ":" V ;
FieldList      = [ Field { "," Field } [ "," ] ] ;
```

## Summary

| Macro              | Chips   | Shape                         | Field order | Attrs | Generates                                           |
| ------------------ | ------- | ----------------------------- | ----------- | ----- | --------------------------------------------------- |
| `led_strip!`       | rp, esp | named device                  | any         | no    | struct + statics + task + methods                   |
| `led2d!`           | rp, esp | named device                  | any         | no    | struct + statics + task + methods                   |
| `led_strips!`      | rp      | group, pre-name fields        | any         | no    | group struct + member structs + PIO split fns       |
| `led2d_from_strip!`| rp      | internal (used by `led_strips!`) | fixed    | no    | 2D wrapper over a strip type                        |
| `pio_split!`       | rp      | positional expr `p.PIOn`      | n/a         | n/a   | call to `pioN_split`                                |
| `led!`             | rp, esp | named device                  | any         | no    | struct + task                                       |
| `button_watch!`    | rp, esp | named device                  | fixed (1)   | yes   | struct + statics + task + methods                   |
| `audio_player!`    | rp, esp | named device                  | any         | no    | struct + `<Name>Playable` + statics + task          |
| `pcm_clip!`        | core    | named data                    | fixed       | no    | `mod <Name>` of consts (WAV compiled in)            |
| `adpcm_clip!`      | core    | named data                    | fixed       | no    | `mod <Name>` of consts (WAV compiled in)            |
| `tone!`            | core    | positional expr               | n/a         | n/a   | PCM clip value                                      |
| `servo!` (rp)      | rp      | **unnamed** keyword expr      | any         | n/a   | servo value                                         |
| `servo!` (esp)     | esp     | named device, **no VIS**      | any         | no    | struct                                              |
| `servo_player!` (rp)  | rp   | named device                  | any         | no    | struct + statics + task                             |
| `servo_player!` (esp) | esp  | named device, **no VIS**      | fixed       | no    | `pub` struct + link-time claim statics + task       |
| `combine!`         | rp, esp | positional variadic expr      | n/a         | n/a   | const-concatenated step array                       |
| `ir!`              | rp, esp | named device                  | fixed       | yes   | type alias over a hidden `irs!` group               |
| `irs!`             | rp, esp | group, pre-name fields        | n/a         | no    | up to 4 structs, always `pub`                       |
| `ir_kepler!`       | rp, esp | named device                  | fixed       | yes   | type alias over hidden group                        |
| `ir_keplers!`      | rp, esp | group, pre-name fields        | n/a         | no    | up to 4 structs                                     |
| `ir_mapping!`      | rp, esp | named device                  | fixed       | yes   | type alias over hidden group                        |
| `ir_mappings!`     | rp, esp | group, pre-name fields        | n/a         | no    | up to 4 structs                                     |
| `lcd_text!`        | rp, esp | named device, pre-name fields | fixed       | no    | struct via hidden `i2cs!` group                     |
| `i2cs!`            | rp, esp | group, pre-name fields        | fixed       | no    | group struct + member structs                       |
| `init_and_start!`  | esp     | positional + keyword stmt     | enumerated  | n/a   | `let` bindings                                      |
| `tga!` (`__cyd_tga`) | core  | positional expr               | n/a         | n/a   | image value                                         |

Internal-only, out of scope: all `__*` helpers, `servo_pin_map!`,
`impl_wifi_pio!`, `impl_dma_irq_map_all_irqs!`, `__impl_wifi_auto_fields!`.

## Grammars

### Named devices (common shape)

Most device macros share this shape and differ only in their field schema:

```ebnf
NamedDevice = { ATTR } VIS NAME "{" FieldList "}" ;
```

`{ ATTR }` is accepted only by `button_watch!` and the IR family.

Field schemas (R = required, O = optional with default):

**`led_strip!` (rp)** any order

| Field         | Kind     | R/O | Default                |
| ------------- | -------- | --- | ---------------------- |
| `pin`         | `PERIPH` | R   |                        |
| `len`         | `EXPR`   | R   |                        |
| `pio`         | `PERIPH` | O   | `PIO0`                 |
| `dma`         | `PERIPH` | O   | `DMA_CH0`              |
| `max_current` | `EXPR`   | O   | `Current::Milliamps(250)` |
| `gamma`       | `EXPR`   | O   | `Gamma::Srgb`          |
| `max_frames`  | `EXPR`   | O   | `16`                   |

**`led_strip!` (esp)** any order: `pin` R, `len` R, `max_current` O 250 mA,
`engine` O (`Engine::Rmt` if chip has RMT else `Engine::Spi`), `gamma` O,
`max_frames` O 16, `reset_us` O 60 (SPI only).

**`led2d!` (rp)** any order: `pin` R, `led_layout` R, `font` R, `pio` O `PIO0`,
`dma` O `DMA_CH0`, `max_current` O, `gamma` O, `max_frames` O 16. Length is
derived from `led_layout`.

**`led2d!` (esp)** any order: `pin` R, `len` R, `led_layout` R, `font` R,
`max_current` O, `engine` O `Engine::Rmt`, `gamma` O, `max_frames` O 16.

**`led!`** (rp, esp) any order: `pin` R `PERIPH`, `max_steps` O 32.

**`button_watch!`** (rp, esp): `pin` R `PERIPH`. The single field must have a
trailing comma (`pin: PIN_13,`).

**`audio_player!` (rp)** any order: `data_pin` R, `bit_clock_pin` R,
`word_select_pin` R, `sample_rate_hz` R `EXPR`, `pio` O `PIO0`, `dma` O
`DMA_CH0`, `max_clips` O 16, `max_volume` O `Volume::MAX`, `initial_volume` O.

**`audio_player!` (esp)** same, but `i2s` O `I2S0` replaces `pio`, and `dma`
is **required**.

**`servo!` (esp)** any order, no `VIS`: `pin` R, `timer` R, `channel` R,
`min_us` O 500, `max_us` O 2500, `max_degrees` O 180, `direction` O
`Direction::Forward`.

**`servo_player!` (esp)** fixed order, no `VIS` (always `pub`, despite docs):
`pin`, `timer`, `channel` R, then optional `min_us`, `max_us`, `max_degrees`,
`direction`, `max_steps` (default 16). Emits `#[unsafe(no_mangle)]` claim
statics so duplicate timer/channel use fails at link time.

**`servo_player!` (rp)** any order: `pin` R, `min_us` O, `max_us` O,
`max_degrees` O, `direction` O, `max_steps` O 16. (No `slice`; derived from pin.)

**`ir!`** fixed order: rp `pio` R, `pin` R; esp `pin` R.
**`ir_kepler!`** fixed order: same as `ir!`.
**`ir_mapping!`** fixed order: (rp `pio` R,) `pin` R, `button` R `TYPE`,
`capacity` R `EXPR`.

The IR singles reject the old form `NAME ":" "{" ... "}"` with a migration
error.

### Named data

```ebnf
PcmClip   = VIS NAME "{"
              "file" ":" EXPR ","
              ( "sample_rate_hz" | "source_sample_rate_hz" ) ":" EXPR
              [ "," "target_sample_rate_hz" ":" EXPR ] [ "," ]
            "}" ;
AdpcmClip = VIS NAME "{"
              "file" ":" EXPR
              [ "," "target_sample_rate_hz" ":" EXPR ] [ "," ]
            "}" ;
```

`target_sample_rate_hz` defaults to the source rate. ADPCM reads the source
rate from the WAV header; PCM requires it.

### Groups

```ebnf
LedStrips   = [ "pio" ":" PERIPH "," ] VIS NAME "{" Member { "," Member } [ "," ] "}" ;
Member      = NAME ":" "{" StripFields [ "," "led2d" ":" "{" Led2dFields "}" ] "}" ;
(* StripFields any order: pin R, len R, max_current R, dma O (auto-numbered
   by member index), gamma O, max_frames O.
   Led2dFields fixed order: led_layout R (must be a const NAME, optionally
   called), [max_frames O], font R. Max 4 members. *)

Irs         = [ "pio" ":" PERIPH "," ] NAME "{" PinMember { "," PinMember } [ "," ] "}" ;
IrKeplers   = Irs ;
IrMappings  = [ "pio" ":" PERIPH "," ] "button" ":" TYPE "," "capacity" ":" EXPR ","
              NAME "{" PinMember { "," PinMember } [ "," ] "}" ;
PinMember   = NAME ":" "{" "pin" ":" PERIPH "}" ;
(* rp requires the pio field; esp has none. 1..4 members. No VIS on group
   or members; members are always pub. *)

I2cs        = "i2c" ":" PERIPH "," "sda_pin" ":" PERIPH "," "scl_pin" ":" PERIPH ","
              VIS NAME "{" Lcd { "," Lcd } [ "," ] "}" ;
Lcd         = VIS NAME "{" "width" ":" EXPR "," "height" ":" EXPR "," "address" ":" EXPR "}" ;
LcdText     = "i2c" ":" PERIPH "," "sda_pin" ":" PERIPH "," "scl_pin" ":" PERIPH ","
              Lcd ;
(* All fixed order. No trailing comma allowed after address. *)
```

### Unnamed keyword expression

```ebnf
ServoRp = FieldList ;
(* any order: pin R EXPR, slice R EXPR, channel O (A | B),
   odd O bool, even O bool (shorthand for channel), min_us O 500,
   max_us O 2500, max_degrees O 180, direction O Forward. *)
```

### Positional

```ebnf
Tone       = EXPR "," EXPR "," EXPR ;                 (* hz, sample_rate_hz, duration *)
Combine    = [ EXPR { "," EXPR } [ "," ] ] ;
Tga        = EXPR [ "," EXPR "," EXPR ] ;             (* path [, width, height] *)
PioSplit   = IDENT "." ( "PIO0" | "PIO1" | "PIO2" ) ;
InitAndStart = IDENT [ "," Opt [ "," Opt ] ] ;
Opt        = "rmt80" ":" IDENT "," "mode" ":" ( "rmt_mode::Blocking" | "rmt_mode::Async" )
           | "ledc" ":" IDENT ;
(* Opt values are names of `let` bindings the macro creates. Each order
   permutation is a separate matcher arm. *)
```

## Inconsistencies

Syntax:

1. **Shared fields go before the name** in `led_strips!`, `irs!`,
   `ir_keplers!`, `ir_mappings!`, `i2cs!`, `lcd_text!`, but inside the braces
   in every other macro.
2. **Members use `Name: { ... }`** in groups, while singles use `Name { ... }`
   and now reject the colon form. The migration was only done halfway.
3. **Field order**: any order for LED/audio/servo (except esp
   `servo_player!`); fixed for IR, clip, LCD, esp `servo_player!`, and the
   `led2d` sub-block.
4. **Trailing commas**: required in `button_watch!`, forbidden after
   `address` in `lcd_text!`, optional elsewhere.
5. **Attributes** accepted only by `button_watch!` and the IR family.
6. **Visibility**: missing from esp `servo!`, esp `servo_player!`, and IR groups/members (always
   `pub`); "required" per comments in `led_strips!` but `$vis:vis` matches
   empty, so it is effectively optional.
7. **`servo!`** is an unnamed expression on rp but a named type on esp.
8. **Aliases**: `source_sample_rate_hz` = `sample_rate_hz` in `pcm_clip!`;
   `odd`/`even` = `channel` in rp `servo!`.
9. **Positional vs keyword**: `tone!`, `tga!`, `pio_split!`, and
   `init_and_start!`'s first argument are positional; the rest is keyword.
10. **Binding-name values**: `init_and_start!`'s `rmt80: rmt80` names a
    generated `let`, unlike every other field, whose value is an input.

Semantics:

11. `max_current` optional (250 mA) in `led_strip!`/`led2d!`, required in
    `led_strips!` members.
12. `dma` optional on rp `audio_player!`, required on esp.
13. `pio` defaults to `PIO0` everywhere except rp IR macros, where it is
    required.
14. Animation length is `max_frames` (LED strips, default 16), `max_steps`
    (`led!` default 32, `servo_player!` default 16).
15. esp `led2d!` requires `len` (redundant with `led_layout`); rp derives it.
16. esp `led2d!` defaults `engine` to `Rmt`; esp `led_strip!` picks by chip.
17. `pcm_clip!` requires a source rate; `adpcm_clip!` reads it from the file.

Implementation:

18. Only rp `led_strip!`, rp `led2d!`, and both `audio_player!`s use the shared
    `__validate_keyword_fields_expr!` (unknown fields via an allow-list macro,
    duplicates via `mod` name collisions). Others give whatever error the
    matcher produces.
19. Defaults are filled by per-field tt-muncher arms; `led_strips!` is about
    1,700 lines, ten arms of which only auto-number default DMA channels.
20. `init_and_start!` enumerates every permutation of its optional fields and
    uses `.expect(...)`, which `AGENTS.md` forbids in MCU app paths.
21. `pcm_clip!` has a duplicated matcher arm.
22. Docs and matchers disagree: esp `servo_player!` documents
    `[<visibility>]` and implies any order; it accepts neither.
23. Resource exclusivity is enforced three different ways: link-time
    `no_mangle` symbols (esp servo), const asserts (LCD addresses), and
    nothing (most PERIPH fields rely on the HAL's ownership).
24. Generated names are derived with `paste!` (`<name>_task`,
    `<NAME>_FRAME_SIGNAL`, ...), invisible in the invocation.

## Implications for const-structures

- **Devices are not data.** Most macros generate a struct, statics, an
  Embassy task, and methods. The "value tree to const struct" model covers
  only clips and the servo expression. The framework needs a codegen hook:
  parse and validate generically, then hand a typed, normalized spec to
  device-specific generation code.
- **`PERIPH` is its own value kind.** Peripheral names become type paths, so
  a schema must distinguish "identifier pasted into a type" from "expression".
- **Groups are a real feature.** `led_strips!`, `irs!`, `i2cs!` need named
  members, cross-member constraints (max 4, unique addresses), and per-member
  defaults that depend on member index (DMA auto-numbering).
- **Schema location.** Field schemas could plausibly be ordinary Rust structs
  with attributes, but the generated code depends on chip-specific HAL types,
  so the schema crate cannot see them; schemas likely live in the proc-macro
  crate as data.

## Regularization questions for Carl

1. Shared group fields: move inside the braces (`Group { pio: PIO0, Members... }`)?
2. Members: `Name { ... }` everywhere, dropping the colon?
3. Any order, trailing comma optional, attributes and visibility allowed,
   everywhere?
4. Drop aliases (`source_sample_rate_hz`, `odd`/`even`)?
5. Make rp `servo!` a named device like esp, or make esp unnamed?
6. Unify `max_frames`/`max_steps` and the defaults listed in 11-17?
7. Keep positional macros (`tone!`, `combine!`, `tga!`, `pio_split!`) out of
   scope? They are expressions, not declarations.
8. `init_and_start!`: convert to keyword-only, or leave as a special case?
