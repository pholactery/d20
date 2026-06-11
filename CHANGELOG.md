# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.1] - 2026-06-11

### Added
- Declared a minimum supported Rust version of `1.85` via `rust-version` in
  `Cargo.toml`. This is the version required by edition 2024 and `rand 0.10`;
  declaring it gives users on older toolchains a clear error instead of a
  cryptic build failure. No code or behavior changes.

## [0.2.0] - 2026-06-10

Modernization to current idiomatic Rust, dependency updates, and a set of
correctness fixes. This is a breaking release: the public API has changed.

### Added
- `D20Error`, a descriptive error enum (`EmptyExpression`, `InvalidTerm`,
  `MissingOperator`, `ZeroSidedDie`, `DiceCountTooLarge`, `SidesTooLarge`,
  `ModifierTooLarge`, `InvalidRange`) returned by all fallible functions.
- Documented, public limits `MAX_DICE`, `MAX_SIDES`, and `MAX_MODIFIER`; values
  beyond them return a clear error instead of overflowing.
- `TermResult` enum (`Dice { multiplier, sides, rolls }` / `Modifier(i32)`) with
  `subtotal()` and `rolls()` accessors, now the element type of `Roll::terms`.
- `roll_dice_with_rng` and `roll_range_with_rng` for deterministic, seeded rolls
  with a caller-supplied RNG.
- `Roll::rolls()`, an explicit iterator of fresh re-rolls that reuses the parsed
  terms instead of re-parsing.
- `d6` shorthand (equivalent to `1d6`) and uppercase `D` (e.g. `1D20`).
- `CHANGELOG.md` (this file).

### Changed
- **Breaking:** `roll_dice` and `roll_range` now return `Result<_, D20Error>`
  instead of `Result<_, &str>`.
- **Breaking:** `Roll.values: Vec<(DieRollTerm, Vec<i8>)>` replaced by
  `Roll.terms: Vec<TermResult>`; `Roll.total` widened from `i32` to `i64`.
- **Breaking:** die-roll term fields widened — multiplier `i8` → `i32`, sides
  `u8` → `u32`, individual die results now `u32`.
- **Breaking:** the surprising infinite `IntoIterator for Roll` is replaced by
  the explicit `Roll::rolls()` method.
- `Roll` now stores the original expression in `drex` (previously a
  whitespace-stripped copy).
- Bumped to Rust edition 2024; updated dependencies `rand 0.3 → 0.10` and
  `regex 0.2 → 1`; the parsing regex is compiled once via `std::sync::LazyLock`.
- The parser now validates the entire input as a sequence of signed terms rather
  than scanning for number-shaped substrings.

### Fixed
- No input can panic anymore. Previously, oversized counts/sides/modifiers
  (e.g. `128d6`, `1d256`, `+500`), `1d0`, `-128d6`, and `roll_range(_, i32::MAX)`
  all panicked despite the advertised `Result` API.
- Whitespace between terms no longer merges them: `2d6 5` is now rejected
  (`MissingOperator`) rather than silently becoming a single 65-sided die.
- Expressions containing stray text (e.g. `I have 5 apples`, `2 plus 2 ...`) are
  now rejected instead of having numbers extracted from them.
- `d6` is rolled as one six-sided die instead of being misread as the
  constant `+6`.
- Uppercase `D` (e.g. `1D20`) now parses correctly instead of panicking.

### Removed
- **Breaking:** `DieRollTerm` is no longer part of the public API (it is now an
  internal parse-only type); inspect results through `Roll::terms` /
  `TermResult` instead.

## [0.1.0] - 2017

Initial release.
