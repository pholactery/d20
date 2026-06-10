//! Characterization tests for the d20 public API.
//!
//! These tests pin the **current, observed behavior** of the library as of the
//! start of the modernization work (edition 2015, rand 0.3, regex 0.2). They are
//! intentionally split into three groups:
//!
//!   1. PRESERVE  — correct behavior that the modernization MUST keep.
//!   2. PANICS    — inputs that currently `panic!` but SHOULD return `Err`
//!                  (defects C1-C7 from the review). Pinned with `#[should_panic]`.
//!   3. WRONG-OK  — inputs that currently return a confidently-wrong `Ok`
//!                  (defects C8-C13). Pinned by asserting the *current wrong*
//!                  output, with a comment describing the intended fix.
//!
//! As each fix lands in a later phase, the corresponding test here is rewritten
//! from "documents the bug" to "asserts the fix", so the diff proves the change.

use d20::roll_dice;
use d20::roll_range;

// ---------------------------------------------------------------------------
// GROUP 1 — PRESERVE: correct behavior that must survive the rewrite.
// ---------------------------------------------------------------------------

#[test]
fn preserve_simple_total_in_range() {
    // 3d6 ranges 3..=18, plus a +4 modifier => 7..=22.
    let r = roll_dice("3d6 + 4").unwrap();
    assert!(r.total >= 7 && r.total <= 22, "got {}", r.total);
}

#[test]
fn preserve_deterministic_one_sided_dice() {
    // 1-sided dice always roll 1, so totals are exact and stable.
    assert_eq!(roll_dice("3d1 + 2d1 + 1").unwrap().total, 6);
    assert_eq!(roll_dice("1d1-3").unwrap().total, -2);
    // Negative multiplier negates that term: -3d1 = -3, +2d1 = +2, +1 => 0.
    assert_eq!(roll_dice("-3d1 + 2d1 + 1").unwrap().total, 0);
}

#[test]
fn preserve_values_structure() {
    let r = roll_dice("2d6 + 6 + 4d10").unwrap();
    assert_eq!(r.values.len(), 3);
    assert_eq!(r.values[0].1.len(), 2); // two d6 rolls
    assert_eq!(r.values[1].1.len(), 1); // single modifier value
    assert_eq!(r.values[2].1.len(), 4); // four d10 rolls
}

#[test]
fn preserve_display_formatting() {
    let roll = roll_dice("3d1 + 5").unwrap();
    assert_eq!(format!("{}", roll), "3d1[1, 1, 1]+5 (Total: 8)");

    let bigger = roll_dice("3d1 - 2d1 - 4").unwrap();
    assert_eq!(format!("{}", bigger), "3d1[1, 1, 1]-2d1[1, 1]-4 (Total: -3)");
}

#[test]
fn preserve_range_roll_bounds() {
    let v = roll_range(1, 100).unwrap();
    assert!(v >= 1 && v <= 100, "got {}", v);
    // Single-value range returns that value.
    assert_eq!(roll_range(3, 3).unwrap(), 3);
}

#[test]
fn preserve_range_rejects_inverted() {
    assert!(roll_range(12, 1).is_err());
}

#[test]
fn preserve_iterator_take_yields_n_rolls() {
    // The current `IntoIterator` yields an *infinite* re-roll stream; `take(n)`
    // bounds it. Phase 6 replaces this with a named iterator method, but the
    // ability to produce N successive rolls must be preserved.
    let v: Vec<_> = roll_dice("3d6").unwrap().into_iter().take(6).collect();
    assert_eq!(v.len(), 6);
    for roll in &v {
        assert!(roll.total >= 3 && roll.total <= 18, "got {}", roll.total);
    }
}

#[test]
fn preserve_non_numeric_garbage_errors() {
    // Phrases with no digit at all correctly produce an error today, and must
    // continue to. (Contrast with C9/C10 below, where digits sneak through.)
    assert!(roll_dice("two plus two equals CHICKEN!").is_err());
}

// ---------------------------------------------------------------------------
// GROUP 2 — formerly PANICS (defects C1-C7), now FIXED in Phases 3-4.
// Inputs that used to abort the thread now either roll correctly (large values
// are supported up to documented caps) or return a clean `Err`.
// ---------------------------------------------------------------------------

#[test]
fn c1_large_multiplier_now_supported() {
    // FIXED (Phase 4): multiplier widened i8 -> i32 with a MAX_DICE cap.
    // "128d6" now rolls 128 dice instead of panicking on i8 overflow.
    let r = roll_dice("128d6").unwrap();
    assert_eq!(r.values[0].1.len(), 128);
    assert!(r.total >= 128 && r.total <= 768, "got {}", r.total);
}

#[test]
fn c2_large_modifier_now_supported() {
    // FIXED (Phase 4): Modifier widened i8 -> i32 with a MAX_MODIFIER cap.
    let r = roll_dice("+500").unwrap();
    assert_eq!(r.total, 500);
}

#[test]
fn c3_sides_over_255_now_supported() {
    // FIXED (Phase 4): sides widened u8 -> u32. "1d256" rolls a 256-sided die.
    let r = roll_dice("1d256").unwrap();
    assert!(r.total >= 1 && r.total <= 256, "got {}", r.total);
}

#[test]
fn c4_sides_128_to_255_now_supported() {
    // FIXED (Phase 4): the lossy `sides as i8` cast is gone; sampling is done in
    // u32, so "1d200" rolls a real 200-sided die.
    let r = roll_dice("1d200").unwrap();
    assert!(r.total >= 1 && r.total <= 200, "got {}", r.total);
}

#[test]
fn c5_zero_sided_die_now_errors() {
    // FIXED (Phase 4): a 0-sided die is a clean error instead of an empty-range panic.
    assert!(roll_dice("1d0").is_err());
}

#[test]
fn c6_min_multiplier_now_supported() {
    // FIXED (Phase 4): `multiplier.unsigned_abs()` replaces the panicking i8 abs;
    // "-128d6" rolls 128 dice and subtracts them.
    let r = roll_dice("-128d6").unwrap();
    assert_eq!(r.values[0].1.len(), 128);
    assert!(r.total >= -768 && r.total <= -128, "got {}", r.total);
}

#[test]
fn c7_roll_range_max_no_longer_overflows() {
    // FIXED in Phase 3: migrating `gen_range(min, max + 1)` to the idiomatic
    // `random_range(min..=max)` removes the `max + 1` overflow. roll_range now
    // samples the full i32 range without panicking.
    let v = roll_range(0, i32::MAX).unwrap();
    assert!(v >= 0, "got {}", v);
}

// ---------------------------------------------------------------------------
// GROUP 3 — WRONG-OK: silently incorrect results (defects C8-C13).
// These assert the CURRENT (wrong) behavior. Post-fix, they assert the
// corrected behavior (a different valid roll, or an `Err`).
// ---------------------------------------------------------------------------

#[test]
fn c8_whitespace_merges_tokens() {
    // "2d6 5" should be 2d6 + 5. Today `split_whitespace().collect()` fuses it
    // into "2d65" => a single 65-sided-die term. CURRENT (wrong) behavior:
    let r = roll_dice("2d6 5").unwrap();
    assert_eq!(r.values.len(), 1, "merged into one term");
    assert_eq!(r.drex, "2d65"); // C12: drex holds the mangled string too
    assert!(r.total >= 2 && r.total <= 130, "rolls a single d65: {}", r.total);
}

#[test]
fn c9_garbage_with_digit_succeeds() {
    // "I have 5 apples" should error; today it extracts "5" and returns Ok(5).
    let r = roll_dice("I have 5 apples").unwrap();
    assert_eq!(r.total, 5);
}

#[test]
fn c10_digit_nonsense_succeeds() {
    // The companion to the GROUP 1 error test: swap the spelled-out "two" for
    // the digit "2" and the same nonsense now SUCCEEDS. Demonstrates the error
    // test passes only by accident (no digits), not by real validation.
    let r = roll_dice("2 plus 2 equals CHICKEN!").unwrap();
    assert_eq!(r.total, 4);
}

#[test]
fn c11_d6_shorthand_parsed_as_modifier() {
    // "d6" (common shorthand for 1d6) should roll one 6-sided die (1..=6).
    // Today the regex needs a leading digit, so it matches only "6" => +6.
    let r = roll_dice("d6").unwrap();
    assert_eq!(r.values.len(), 1);
    assert_eq!(r.total, 6, "treated as constant +6, never rolled");
}

#[test]
fn c13_leading_plus_dropped_in_signed_pair() {
    // "+-3": the regex grabs "-3" and silently drops the leading '+'.
    let r = roll_dice("+-3").unwrap();
    assert_eq!(r.total, -3);
}

// ---------------------------------------------------------------------------
// GROUP 4 — documented caps (Phase 4). Values beyond the supported limits are
// rejected with a descriptive error instead of panicking or hanging.
// ---------------------------------------------------------------------------

use d20::D20Error;
use d20::{MAX_DICE, MAX_SIDES};

#[test]
fn caps_reject_too_many_sides() {
    let err = roll_dice(&format!("1d{}", MAX_SIDES as u64 + 1)).unwrap_err();
    assert!(matches!(err, D20Error::SidesTooLarge { .. }), "got {err:?}");
}

#[test]
fn caps_reject_too_many_dice() {
    let err = roll_dice(&format!("{}d6", MAX_DICE as u64 + 1)).unwrap_err();
    assert!(matches!(err, D20Error::DiceCountTooLarge { .. }), "got {err:?}");
}

#[test]
fn caps_reject_unparseable_huge_number() {
    // A digit run too large for even a 64-bit integer is a clean InvalidTerm,
    // not a panic.
    let err = roll_dice("1d999999999999999999999999").unwrap_err();
    assert!(matches!(err, D20Error::InvalidTerm(_)), "got {err:?}");
}

#[test]
fn caps_zero_sided_die_is_specific_error() {
    assert_eq!(roll_dice("1d0").unwrap_err(), D20Error::ZeroSidedDie);
}

#[test]
fn caps_no_input_panics_across_extreme_values() {
    // Sweep a range of hostile inputs; every one must return Ok or Err, never panic.
    let inputs = [
        "0d6", "1d1", "1000000d1000000", "-1000000d20", "+1000000", "-1000000",
        "1d1000001", "1000001d6", "999999999999d6", "1d0", "",
    ];
    for inp in inputs {
        let _ = roll_dice(inp); // must not panic
    }
}
