# Device Envoy Macro Migration

<!-- TODO0 consider deleting this survey once the migration has shipped and the historical baseline is no longer useful. -->

This document records how Device Envoy's declaration macros moved to
`const-structures`. It covers what exists now, what changed against the
pre-migration baseline, the bugs the migration uncovered, and which macros were
intentionally left as hand-written `macro_rules!`.

- The framework's language reference is `src/define.md` (the rustdoc of
  `define!`); its architecture and design rationale are in
  [CONST_STRUCTURES_SPEC.md](CONST_STRUCTURES_SPEC.md).
- Sections marked **Historical** describe Device Envoy as it was at the
  baseline, `main` at `4c1ee16f`. They are kept for comparison and do not
  describe current behavior.
- The migration is on the Device Envoy branch `proc-macro-const-structures`
  (through `446a666b`). It has not been merged to `main`.

## Outcome

| | Baseline (`4c1ee16f`) | Now |
| --- | --- | --- |
| Parsing caller syntax | Hand-written `macro_rules!` matchers per macro, mostly tt-munchers | None. Every declaration macro is a `define!` schema |
| Generating code | Hand-written `macro_rules!` generator per macro | A `generate { ... }` template beside each schema; no `=> generator` escape hatch in use |
| Validation | Only four macros used a shared field validator; others failed with whatever error the matcher produced | Every macro: unknown, duplicate, missing, wrong-kind, and member-count errors at the caller's tokens |
| Syntax and field docs | Hand-written `**Syntax:**` blocks that drifted from the matchers | Generated from the schema |
| Instance docs | Ad hoc | Generated configuration table on every declared item |
| Separate proc-macro crate | No (pure `macro_rules!`) | No. A transitional `device-envoy-macros` crate was created and then deleted once the alias design let schemas live in the library |
| Macro source in `crates/*/src` | 14,188 lines in `macro_rules!` | 1,912 lines in `macro_rules!` plus 3,246 lines in `define!` blocks |

The line counts measure the text inside `macro_rules! { ... }` and
`const_structures::define! { ... }` blocks under `crates/*/src`, excluding
examples, tests, and xtasks. The `define!` count includes the macros'
hand-written docs, which at the baseline sat outside the `macro_rules!` braces
and were not counted. The largest baseline generators were `__led_strips_impl`
(1,689 lines), `__ir_mappings_impl` (1,199), `__servo_player_impl` (1,113),
`__audio_player_impl` (1,112), and `__led2d_impl` (1,062).

## Current schemas

All 33 declaration macros use `define!` with a `generate { ... }` template.
Each schema is in the module whose API it generates. The RP and ESP macros are
also re-exported at their crate roots. The core clip macros are re-exported
from each platform crate's `audio_player` module. Shared group fields go inside the group's
braces, and members are written `Name { ... }`.

| Macro | Crate | Fields (`?` optional, `=` defaulted) | Members |
| --- | --- | --- | --- |
| `pcm_clip!` | core | `file`, `source_sample_rate_hz`, `target_sample_rate_hz?` | |
| `adpcm_clip!` | core | `file`, `target_sample_rate_hz?` | |
| `audio_player!` | rp | `data_pin`, `bit_clock_pin`, `word_select_pin`, `sample_rate_hz`, `pio = PIO0`, `dma = DMA_CH0`, `max_clips = 16`, `max_volume =`, `initial_volume =` | |
| `audio_player!` | esp | same, with `i2s = I2S0` instead of `pio`, and `dma` required | |
| `button_watch!` | rp, esp | `pin` | |
| `led!` | rp, esp | `pin`, `max_steps = 32` | |
| `led_strips!` | rp | `pio = PIO0` | 1..=4: `pin`, `len`, `max_current`, `dma = by_index[DMA_CH0..DMA_CH3]`, `gamma =`, `max_frames = 16`, `led2d?: { led_layout, font }` |
| `led_strip!` | rp | `pin`, `len`, `pio = PIO0`, `dma = DMA_CH0`, `max_current =`, `gamma =`, `max_frames = 16` | |
| `led2d!` | rp | `pin`, `led_layout`, `font`, `pio = PIO0`, `dma = DMA_CH0`, `max_current =`, `gamma =`, `max_frames = 16` | |
| `led_strip!` | esp | `pin`, `len`, `max_current =`, `engine?`, `gamma =`, `max_frames = 16`, `reset_us?` | |
| `led2d!` | esp | `pin`, `len`, `led_layout`, `font`, `max_current =`, `engine?`, `gamma =`, `max_frames = 16` | |
| `irs!` / `ir!` | rp | `pio` (group or single), `pin` (single) | 1..=4: `pin` |
| `irs!` / `ir!` | esp | `pin` (single) | 1..=4: `pin` |
| `ir_keplers!` / `ir_kepler!` | rp, esp | as `irs!` / `ir!` | 1..=4: `pin` |
| `ir_mappings!` / `ir_mapping!` | rp | `pio`, `button: ty`, `capacity`, `pin` (single) | 1..=4: `pin` |
| `ir_mappings!` / `ir_mapping!` | esp | `button: ty`, `capacity`, `pin` (single) | 1..=4: `pin` |
| `i2cs!` | rp, esp | `i2c`, `sda_pin`, `scl_pin` | 1..: `width`, `height`, `address` |
| `lcd_text!` | rp, esp | `i2c`, `sda_pin`, `scl_pin`, `width`, `height`, `address` | |
| `servo!` | rp | `pin`, `min_us = 500`, `max_us = 2500`, `max_degrees = 180`, `direction = Forward` | |
| `servo!` | esp | `pin`, `timer`, `channel`, then as rp | |
| `servo_player!` | rp | as rp `servo!`, plus `max_steps = 16` | |
| `servo_player!` | esp | as esp `servo!`, plus `max_steps = 16` | |

Defaults written `=` with no value are defaults whose docs use
`#[default_display]` (for example `Current::Milliamps(250)`, `Gamma::Srgb`,
`Volume::MAX`). The schemas themselves are authoritative. Each macro's rustdoc
page shows its generated field table.

### How the templates are organized

- **A single device as a one-member group.** RP `led_strip!`, `led2d!`, `ir!`,
  `ir_kepler!`, `ir_mapping!`, and both chips' `lcd_text!` render a call to their
  group macro with one member, passing `$decl.attrs #[doc = $decl.doc]` on the
  member. The RP LED and IR singles add a `new` that unpacks the group's
  one-element tuple. `lcd_text!` needs none, because each `i2cs!` member already
  has its own `new`. By the written-doc
  rule, the member then carries the single macro's own description. The
  synthetic group is `#[doc(hidden)]`.
- **A group that calls its single.** On ESP, every IR receiver owns its RMT
  channel and task, so a receiver is self-contained. `ir!`, `ir_kepler!`, and
  `ir_mapping!` templates emit the whole receiver. `irs!`, `ir_keplers!`, and
  `ir_mappings!` loop `$for` over members, invoke the single macro per member,
  and add a group constructor.
- **Direct.** `audio_player!`, `button_watch!`, `led!`, `servo!`,
  `servo_player!`, `i2cs!`, and the clip macros emit their items directly.
- **Template plus a value-dispatch helper.** ESP `led_strip!` and `led2d!`
  forward their resolved fields to `__led_engine_normalize!`, which reduces an
  `engine` path such as `Engine::Spi` to `Spi` or `Rmt` and dispatches by chip
  capability. That choice depends on a value's spelling, which the template
  language intentionally does not express.
- **Template plus a shared emission helper.** RP `led_strips!` calls
  `__led_strip_type!` from both branches of `$if let Some(panel) = $strip.led2d`
  (a plain strip, or the strip behind a panel), and `led2d_from_strip!` for the
  panel wrapper.

## API regularization

Each numbered item is an inconsistency found in the baseline survey (see
[Historical: baseline inconsistencies](#historical-baseline-inconsistencies)).
Items 1–10 were syntax, 11–17 semantics, and 18–24 implementation.

| # | Baseline | Now |
| --- | --- | --- |
| 1 | Shared group fields before the group name in six macros | Inside the braces everywhere |
| 2 | Members `Name: { ... }`, singles `Name { ... }` | `Name { ... }` everywhere; the colon form gets "remove the `:`" |
| 3 | Field order fixed in IR, clips, LCD, ESP `servo_player!`, and the `led2d` sub-block | Any order everywhere |
| 4 | Trailing comma required in `button_watch!`, forbidden after `address` in `lcd_text!` | Optional everywhere |
| 5 | Attributes only on `button_watch!` and IR | On every declaration and member |
| 6 | No visibility on ESP `servo!`/`servo_player!` or IR groups (always `pub`) | Visibility on every declaration; members inherit the group's, and writing one on a member is an error |
| 7 | RP `servo!` an unnamed expression, ESP `servo!` a named type | Both named declarations. On RP the PWM slice is passed to `new`, and a trait bound checks that it is the pin's slice |
| 8 | Aliases `sample_rate_hz` / `source_sample_rate_hz`, `odd` / `even` / `channel` | One spelling: `source_sample_rate_hz`. The RP servo channel fields went away with item 7 |
| 9 | `tone!`, `tga!`, `pio_split!`, `init_and_start!` positional | `pio_split!` removed (unused). The others remain; they are not declarations |
| 10 | `init_and_start!` fields name generated `let` bindings | Unchanged |
| 11 | `max_current` required in `led_strips!` members, defaulted elsewhere | Intentional: one device defaults to 250 mA; a group of several must budget each member, because currents on a shared supply add up. The field docs now say so, and that separately declared devices each get the default |
| 12 | `dma` defaulted on RP `audio_player!`, required on ESP | Unchanged |
| 13 | `pio` defaults to `PIO0` except RP IR, where it is required | Unchanged |
| 14 | Animation length named `max_frames` (LED strips) or `max_steps` (`led!`, `servo_player!`) | Names unchanged. All LED strip and panel schemas share `MAX_FRAMES_DEFAULT` (16) |
| 15 | ESP `led2d!` requires `len` although `led_layout` implies it | Unchanged |
| 16 | ESP `led2d!` defaulted `engine` to RMT, while `led_strip!` chose by chip | Both choose by chip: RMT where available, otherwise SPI, through the shared capability check |
| 17 | `pcm_clip!` requires a source rate; `adpcm_clip!` reads it from the WAV header | Unchanged: a raw PCM file has no header |
| 18 | Shared field validator in only four macros | Removed; the framework validates every macro |
| 19 | Defaults filled by tt-muncher arms (`led_strips!` about 1,700 lines) | Defaults are schema data |
| 20 | `init_and_start!` enumerates each permutation of its options and calls `.expect(...)` | Unchanged; out of scope |
| 21 | Duplicated matcher arm in `pcm_clip!` | Gone with the matcher |
| 22 | ESP `servo_player!` docs advertised visibility and any order; the matcher accepted neither | Both accepted, and the syntax docs are generated |
| 23 | Resource exclusivity enforced three ways | Still varied: ESP servo link-time claim symbols, LCD unique-address const assert, and now a compile error for two RP LED groups on one PIO in one module. Other peripherals rely on HAL ownership |
| 24 | Generated item names derived with `paste!` | Derived with `$ident` / `$snake` / `$upper` in templates; still not visible in the invocation |

The items marked "Unchanged" in 12–17 are open questions about Device Envoy's
APIs, not framework limitations. Each schema owns its own field meanings and
defaults.

## Bugs found during the migration

### Existing defects the migration exposed

- **RP compile-only tests were not running.** `check-compile-only` looked for
  `tests-compile-only` under the workspace root instead of the RP crate, and
  `check-all` skipped silently when the directory was missing. Both now use the
  crate root, and a missing directory is a failure (`86577bfc`).
- **ESP embedded tests were not running.** `check-embedded-tests` searched the
  wrong directory (`b0205d5d`).
- **`check-demos` used a stale path** (`6fe7ff34`).
- **The Xtensa linker received `-Tlinkall.x` twice**, from both the root and
  the crate Cargo configuration (`b0205d5d`).
- **ESP panels defaulted to RMT** even on chips without RMT. Default panels now
  select SPI on such chips, and explicit RMT requests go through the shared
  capability check (item 16).
- **ESP `servo_player!` documentation and matcher disagreed** (item 22).

### Regressions the migration introduced and then fixed

- The old `led2d` sub-block's `max_frames` moved to the member level. The
  compile-only tests that relied on 48-frame panels were updated to set it
  there (`86577bfc`).
- Hand-written macro docs for ESP `led2d!` (`29590956`) and RP
  `servo_player!` (`446a666b`) ended up on a hidden generator instead of the
  public macro, because `///` comments attach across blank lines to the next
  item. Both now sit in their `define!`.

### Framework defects found by this client

- Absolute-path access to the macro-expanded export is rejected by rustc,
  which led to the wrapper-plus-alias design.
- Rustdoc drops an intermediate re-export's docs across crates, so the
  generated syntax and field tables moved onto the wrapper.
- `$vis:vis` cannot match an empty visibility at the end of `[...]`, so an
  inherited visibility is passed as `pub(self)`.
- Embedded clients need a `no_std` facade.
- Snake case split `LED2D` incorrectly; all-caps words with digits now stay
  together.

## Macros intentionally left as `macro_rules!`

**User-facing, not declarations.** They take positional or statement-shaped
input and do not declare a named configuration:

| Macro | Crate | Why it stays |
| --- | --- | --- |
| `tone!` | core | Positional expression: frequency, sample rate, duration |
| `combine!` | rp, esp | Variadic expression concatenating servo step arrays |
| `tga!` (`__cyd_tga`) | core | Positional expression: image path, optional size |
| `init_and_start!` | esp | Creates `let` bindings in the caller's scope; see items 10 and 20 |

**Internal helpers called by templates.** Each has a fixed call shape and is
`#[doc(hidden)]`:

| Helper | Why it is a macro rather than template text |
| --- | --- |
| `__led_engine_normalize!`, `__led_strip_dispatch_*!`, `__led2d_dispatch_engine!`, `__led_strip_inner!`, `__led_strip_impl!`, `__led_strip_spi_*!`, `__led2d_strip_*!`, `__led_strip_first_or_default!` (esp) | Branch on the spelling of `engine` and on chip capability, which templates intentionally cannot do |
| `__led_strip_type!` (rp) | Strip struct, constructor, and task, emitted from both branches of `led_strips!` |
| `led2d_from_strip!` (rp) | Panel wrapper over a strip type; several arms by layout form. A candidate for a future template |

**Internal implementation macros** that generate repetitive impls inside the
crate and are not invoked by users: `__impl_wifi_auto_fields!`, `impl_wifi_pio!`,
`impl_dma_irq_map_all_irqs!`, `servo_pin_map!`, `__define_ir_task!`.

## Historical: baseline survey at `4c1ee16f`

> **Historical.** This section describes Device Envoy before the migration. It
> is preserved to support the comparisons above. None of it describes current
> syntax or behavior.

The survey covered every user-facing `macro_rules!` DSL. Where docs and
matchers disagreed, the matcher was taken as authoritative.

| Macro | Chips | Shape | Field order | Attrs | Generates |
| --- | --- | --- | --- | --- | --- |
| `led_strip!` | rp, esp | named device | any | no | struct, statics, task, methods |
| `led2d!` | rp, esp | named device | any | no | struct, statics, task, methods |
| `led_strips!` | rp | group, fields before name | any | no | group and member structs, PIO split functions |
| `led2d_from_strip!` | rp | internal | fixed | no | 2D wrapper over a strip type |
| `pio_split!` | rp | positional `p.PIOn` | n/a | n/a | call to `pioN_split` |
| `led!` | rp, esp | named device | any | no | struct, task |
| `button_watch!` | rp, esp | named device | fixed (1) | yes | struct, statics, task, methods |
| `audio_player!` | rp, esp | named device | any | no | struct, `<Name>Playable`, statics, task |
| `pcm_clip!`, `adpcm_clip!` | core | named data | fixed | no | `mod <Name>` of consts |
| `tone!` | core | positional | n/a | n/a | PCM clip value |
| `servo!` (rp) | rp | unnamed keyword expression | any | n/a | servo value |
| `servo!` (esp) | esp | named device, no visibility | any | no | struct |
| `servo_player!` (rp) | rp | named device | any | no | struct, statics, task |
| `servo_player!` (esp) | esp | named device, no visibility | fixed | no | `pub` struct, link-time claims, task |
| `combine!` | rp, esp | positional variadic | n/a | n/a | step array |
| `ir!`, `ir_kepler!`, `ir_mapping!` | rp, esp | named device | fixed | yes | alias over a hidden group |
| `irs!`, `ir_keplers!`, `ir_mappings!` | rp, esp | group, fields before name | n/a | no | up to 4 structs, always `pub` |
| `lcd_text!` | rp, esp | named device, fields before name | fixed | no | struct via hidden `i2cs!` group |
| `i2cs!` | rp, esp | group, fields before name | fixed | no | group and member structs |
| `init_and_start!` | esp | positional and keyword statement | enumerated | n/a | `let` bindings |
| `tga!` | core | positional | n/a | n/a | image value |

Representative baseline grammars (EBNF; `[x]` optional, `{x}` repeated):

```ebnf
LedStrips  = [ "pio" ":" PERIPH "," ] VIS NAME "{" Member { "," Member } [ "," ] "}" ;
Member     = NAME ":" "{" StripFields [ "," "led2d" ":" "{" Led2dFields "}" ] "}" ;
IrMappings = [ "pio" ":" PERIPH "," ] "button" ":" TYPE "," "capacity" ":" EXPR ","
             NAME "{" PinMember { "," PinMember } [ "," ] "}" ;
I2cs       = "i2c" ":" PERIPH "," "sda_pin" ":" PERIPH "," "scl_pin" ":" PERIPH ","
             VIS NAME "{" Lcd { "," Lcd } [ "," ] "}" ;
ServoRp    = FieldList ;   (* pin, slice, channel | odd | even, min_us, ... *)
```

### Historical: baseline inconsistencies

<!-- markdownlint-disable MD029 -- numbering is referenced from the regularization table -->

Syntax:

1. Shared fields went before the name in `led_strips!`, `irs!`, `ir_keplers!`,
   `ir_mappings!`, `i2cs!`, and `lcd_text!`, but inside the braces elsewhere.
2. Group members used `Name: { ... }`; singles used `Name { ... }` and rejected
   the colon form.
3. Field order was any order for LED, audio, and servo (except ESP
   `servo_player!`), and fixed for IR, clips, LCD, ESP `servo_player!`, and the
   `led2d` sub-block.
4. A trailing comma was required in `button_watch!`, forbidden after `address`
   in `lcd_text!`, and optional elsewhere.
5. Only `button_watch!` and the IR family accepted attributes.
6. ESP `servo!`, ESP `servo_player!`, and IR groups and members had no
   visibility (always `pub`). `led_strips!` comments called visibility required,
   but `$vis:vis` matched empty.
7. `servo!` was an unnamed expression on RP and a named type on ESP.
8. `pcm_clip!` accepted `source_sample_rate_hz` or `sample_rate_hz`. RP
   `servo!` accepted `odd` / `even` for `channel`.
9. `tone!`, `tga!`, `pio_split!`, and `init_and_start!`'s first argument were
   positional.
10. `init_and_start!`'s `rmt80: rmt80` named a generated `let` binding, unlike
    every other field.

Semantics:

11. `max_current` was optional (250 mA) in `led_strip!`/`led2d!` and required
    in `led_strips!` members.
12. `dma` was optional on RP `audio_player!` and required on ESP.
13. `pio` defaulted to `PIO0` except in RP IR macros, where it was required.
14. Animation length was `max_frames` (LED strips, default 16) or `max_steps`
    (`led!` default 32, `servo_player!` default 16).
15. ESP `led2d!` required `len`, redundant with `led_layout`; RP derived it.
16. ESP `led2d!` defaulted `engine` to RMT; ESP `led_strip!` chose by chip.
17. `pcm_clip!` required a source rate; `adpcm_clip!` read it from the file.

Implementation:

18. Only RP `led_strip!`, RP `led2d!`, and both `audio_player!`s used the shared
    `__validate_keyword_fields_expr!`.
19. Defaults were filled by per-field tt-muncher arms. `led_strips!` was about
    1,700 lines, ten arms of which only auto-numbered default DMA channels.
20. `init_and_start!` enumerated every permutation of its optional fields and
    used `.expect(...)`, which Device Envoy's `AGENTS.md` forbids in MCU app
    paths.
21. `pcm_clip!` had a duplicated matcher arm.
22. ESP `servo_player!` documented `[<visibility>]` and implied any order; it
    accepted neither.
23. Resource exclusivity was enforced by link-time `no_mangle` symbols (ESP
    servo), const asserts (LCD addresses), or not at all.
24. Generated names were derived with `paste!`, invisible in the invocation.

<!-- markdownlint-enable MD029 -->

### Historical: implications drawn at the time

The baseline survey concluded that devices are not data (most macros generate
structs, statics, tasks, and methods, so the framework needed a code-generation
step after generic validation). It also concluded that peripheral names need
their own value kind, and that groups with named members, member limits, and
index-dependent defaults are a real feature. All three held.

It also guessed that schemas would have to live in a proc-macro crate, because
generated code depends on chip-specific HAL types. That guess was overturned:
the wrapper-plus-alias design puts schemas in the library crates beside the
HAL-dependent code (see the spec's "Why schemas live beside their APIs").
