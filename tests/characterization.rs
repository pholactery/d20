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
// GROUP 2 — PANICS that SHOULD become `Err` (defects C1-C7).
// Each is pinned with `#[should_panic]`. Post-fix, these become
// `assert!(roll_dice(...).is_err())` style tests.
// ---------------------------------------------------------------------------

#[test]
#[should_panic]
fn c1_multiplier_overflow_panics() {
    // multiplier: i8 (max 127). Should instead be a graceful error or supported.
    let _ = roll_dice("128d6");
}

#[test]
#[should_panic]
fn c2_modifier_overflow_panics() {
    // Modifier(i8) (max 127). "+500" should not abort the thread.
    let _ = roll_dice("+500");
}

#[test]
#[should_panic]
fn c3_sides_over_255_panics() {
    // sides: u8 parse overflow. "1d256" should roll a 256-sided die or error.
    let _ = roll_dice("1d256");
}

#[test]
#[should_panic]
fn c4_sides_128_to_255_panics() {
    // sides parses as u8=200 then `200 as i8` wraps to -56 => empty gen_range.
    let _ = roll_dice("1d200");
}

#[test]
#[should_panic]
fn c5_zero_sided_die_panics() {
    // gen_range(1, 1) is an empty range. A 0-sided die should be a clean error.
    let _ = roll_dice("1d0");
}

#[test]
#[should_panic]
fn c6_min_multiplier_abs_panics() {
    // (-128i8).abs() overflows. "-128d6" should roll 128 dice (negated).
    let _ = roll_dice("-128d6");
}

#[test]
#[should_panic]
fn c7_roll_range_max_overflow_panics() {
    // `max + 1` overflows i32 when max == i32::MAX.
    let _ = roll_range(0, i32::MAX);
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
