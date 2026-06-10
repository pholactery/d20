//! Property-based and fuzz tests for the d20 public API.
//!
//! These complement the example-based characterization tests with invariants
//! that must hold across a large, generated input space:
//!
//!   * `never_panics_on_arbitrary_input` — the headline robustness guarantee:
//!     no input string ever panics; every call returns `Ok` or `Err`.
//!   * `total_is_within_theoretical_bounds` — every successful roll's total lies
//!     within `[min possible, max possible]` for the generated expression.
//!   * `same_seed_produces_identical_rolls` — the injected RNG makes rolls fully
//!     reproducible, and the values stay within bounds.

use proptest::prelude::*;
use rand::SeedableRng;
use rand::rngs::StdRng;

/// A term used to build a known-valid expression together with its numeric bounds.
#[derive(Debug, Clone)]
enum Term {
    Die { neg: bool, count: u32, sides: u32 },
    Modifier(i32),
}

fn term_strategy() -> impl Strategy<Value = Term> {
    prop_oneof![
        (any::<bool>(), 1u32..=20, 1u32..=100).prop_map(|(neg, count, sides)| Term::Die {
            neg,
            count,
            sides
        }),
        (-1000i32..=1000).prop_map(Term::Modifier),
    ]
}

/// Builds a valid roll expression string alongside the minimum and maximum
/// totals it can possibly produce. Terms after the first are always joined with
/// an explicit `+`/`-` operator, as the parser requires.
fn expr_strategy() -> impl Strategy<Value = (String, i64, i64)> {
    prop::collection::vec(term_strategy(), 1..6).prop_map(|terms| {
        let mut expr = String::new();
        let (mut min, mut max) = (0i64, 0i64);

        for (i, term) in terms.iter().enumerate() {
            match *term {
                Term::Die { neg, count, sides } => {
                    let (c, s) = (count as i64, sides as i64);
                    if neg {
                        expr.push_str(if i == 0 { "-" } else { " - " });
                        min -= c * s;
                        max -= c;
                    } else {
                        if i > 0 {
                            expr.push_str(" + ");
                        }
                        min += c;
                        max += c * s;
                    }
                    expr.push_str(&format!("{count}d{sides}"));
                }
                Term::Modifier(m) => {
                    let mag = (m as i64).abs();
                    if m < 0 {
                        expr.push_str(if i == 0 { "-" } else { " - " });
                        min -= mag;
                        max -= mag;
                        expr.push_str(&mag.to_string());
                    } else {
                        if i > 0 {
                            expr.push_str(" + ");
                        }
                        min += m as i64;
                        max += m as i64;
                        expr.push_str(&m.to_string());
                    }
                }
            }
        }

        (expr, min, max)
    })
}

proptest! {
    /// The core robustness guarantee: no input — however hostile — may panic.
    #[test]
    fn never_panics_on_arbitrary_input(s in any::<String>()) {
        let _ = d20::roll_dice(&s);
    }

    /// Every successful roll's total must lie within the expression's bounds.
    #[test]
    fn total_is_within_theoretical_bounds((expr, min, max) in expr_strategy()) {
        let mut rng = StdRng::seed_from_u64(0xD20_D1CE);
        let roll = d20::roll_dice_with_rng(&expr, &mut rng)
            .unwrap_or_else(|e| panic!("valid expr {expr:?} failed to parse: {e}"));
        prop_assert!(
            roll.total >= min && roll.total <= max,
            "expr={expr:?} total={} expected within [{min}, {max}]",
            roll.total,
        );
    }

    /// The same seed yields bit-for-bit identical rolls (reproducibility), and a
    /// fresh seed still respects the bounds.
    #[test]
    fn same_seed_produces_identical_rolls((expr, min, max) in expr_strategy(), seed in any::<u64>()) {
        let a = d20::roll_dice_with_rng(&expr, &mut StdRng::seed_from_u64(seed)).unwrap();
        let b = d20::roll_dice_with_rng(&expr, &mut StdRng::seed_from_u64(seed)).unwrap();
        prop_assert_eq!(&a, &b);
        prop_assert!(a.total >= min && a.total <= max);
    }
}

/// A plain (non-proptest) determinism check with a fixed seed, documenting that
/// `roll_dice_with_rng` is reproducible.
#[test]
fn fixed_seed_is_reproducible() {
    let roll =
        |seed| d20::roll_dice_with_rng("3d6 + 2d10 - 4", &mut StdRng::seed_from_u64(seed)).unwrap();
    assert_eq!(roll(7), roll(7));
    // The roll iterator is also seedable through the same parsed terms.
    let first = roll(7);
    assert!(first.total >= (3 + 2 - 4) && first.total <= (18 + 20 - 4));
}
