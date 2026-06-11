//! D20
//!
//! **D20** is a simple crate designed to evaluate _roll expressions_. A _roll expression_ is an
//! english-language string that reflects the intent of a dungeon or game master to perform a
//! particular roll.
//!
//! For example, in a tabletop game you may frequently hear phrases like _"roll 2d10"_, or
//! _"roll 3d6 and add 5"_. These are roll expressions, and the components within them are
//! what we call _die roll terms_. A _die roll term_ is either a term that calls for the rolling
//! of an n-sided die x times (e.g. 3d6) or a modifier that simply adds or subtracts a constant value
//! from the larger expression.
//!
//! Examples of valid _roll expressions_ include:
//!
//! * 3d6
//! * 2d10 + 5
//! * 1d20-3
//! * +6
//! * -2
//! * 3d10+5d100-21+7
//!
//!
//! Roll expressions can have arbitrary length and complexity, and it is perfectly legal for the final result
//! of a roll expression to be negative after applying modifiers.
//!
//! # Examples
//! ```
//! let r = d20::roll_dice("3d6 + 4").unwrap();
//! assert!(r.total > 6);
//! let r = d20::roll_dice("1d1-3").unwrap();
//! assert_eq!(r.total, -2);
//!
//! // Bad expressions produce errors rather than panicking.
//! assert!(d20::roll_dice("roll four chickens and add six ferrets").is_err());
//! ```
//! ### Iterating Roll
//! A valid `Roll` can be turned into an open-ended iterator via its `rolls()` method, providing successive
//! rolls of the given die roll expression.
//!
//! _Note that it will be necessary to constrain the iterator via `take(n)`._
//!
//! ```rust
//! use d20::*;
//!
//! let v: Vec<Roll> = d20::roll_dice("3d6").unwrap().rolls().take(3).collect();
//!
//! assert_eq!(v.len(), 3);
//! assert!(v[0].total >= 3 && v[0].total <= 18);
//! assert!(v[1].total >= 3 && v[1].total <= 18);
//! assert!(v[2].total >= 3 && v[2].total <= 18);
//! ```
//!
//! ### Range Rolls
//! If you are less concerned about dice rolls and require only a random number within a given range, `roll_range()`
//! will do just that.
//!
//! ```rust
//! let rg = d20::roll_range(1, 100).unwrap();
//! assert!((1..=100).contains(&rg));
//! ```
//!
use rand::{Rng, RngExt};
use regex::Regex;
use std::fmt;
use std::sync::LazyLock;

/// Matches a single term anchored at the start of the remaining input, allowing
/// surrounding whitespace. Captures the optional leading sign, and either a die
/// roll (`count` may be empty for shorthand like `d6`) or a numeric modifier.
/// Compiled once on first use and reused for every parse.
static TERM_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(?P<sign>[+-]?)\s*(?:(?P<count>\d*)[dD](?P<sides>\d+)|(?P<modval>\d+))\s*")
        .unwrap()
});

/// Maximum number of dice a single term may roll (e.g. the `100` in `100d6`).
/// Larger counts are rejected with [`D20Error::DiceCountTooLarge`] rather than
/// panicking or hanging.
pub const MAX_DICE: u32 = 1_000_000;

/// Maximum number of sides a die may have (e.g. the `20` in `1d20`). Larger
/// values are rejected with [`D20Error::SidesTooLarge`].
pub const MAX_SIDES: u32 = 1_000_000;

/// Maximum absolute value of a numeric modifier (e.g. the `5` in `+5`). Larger
/// magnitudes are rejected with [`D20Error::ModifierTooLarge`].
pub const MAX_MODIFIER: u32 = 1_000_000;

/// The error type returned when a roll expression cannot be evaluated.
///
/// Every failure mode is reported as a value of this type; the library never
/// panics on malformed or out-of-range input.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum D20Error {
    /// The expression contained no recognizable die-roll terms.
    #[error("invalid die roll expression: no die roll terms found")]
    EmptyExpression,
    /// A term could not be parsed (e.g. a number too large to fit a 64-bit integer),
    /// or trailing/garbage characters were found that are not part of a valid term.
    #[error("invalid term: '{0}'")]
    InvalidTerm(String),
    /// Two terms were placed next to each other without a `+` or `-` operator
    /// between them (e.g. `2d6 5`).
    #[error("missing '+' or '-' operator before '{0}'")]
    MissingOperator(String),
    /// A die was specified with zero sides, which cannot be rolled.
    #[error("invalid die: a die must have at least one side")]
    ZeroSidedDie,
    /// A die-roll term asked for more than [`MAX_DICE`] dice.
    #[error("dice count {count} exceeds the maximum of {max}")]
    DiceCountTooLarge {
        /// The requested number of dice.
        count: u64,
        /// The maximum allowed ([`MAX_DICE`]).
        max: u32,
    },
    /// A die was specified with more than [`MAX_SIDES`] sides.
    #[error("die with {sides} sides exceeds the maximum of {max}")]
    SidesTooLarge {
        /// The requested number of sides.
        sides: u64,
        /// The maximum allowed ([`MAX_SIDES`]).
        max: u32,
    },
    /// A modifier's magnitude exceeded [`MAX_MODIFIER`].
    #[error("modifier {modifier} exceeds the maximum magnitude of {max}")]
    ModifierTooLarge {
        /// The requested modifier value.
        modifier: i64,
        /// The maximum allowed magnitude ([`MAX_MODIFIER`]).
        max: u32,
    },
    /// [`roll_range`] was called with `min` greater than `max`.
    #[error("invalid range: min ({min}) must be less than or equal to max ({max})")]
    InvalidRange {
        /// The supplied lower bound.
        min: i32,
        /// The supplied upper bound.
        max: i32,
    },
}

/// Represents the _results_ of an evaluated die roll expression.
///
/// The `Roll` struct contains the original _die roll expression_ passed to
/// [`roll_dice`], the evaluated result of each term, and the net total.
///
/// `terms` always contains at least one element, because a roll expression is
/// not valid without at least one term. Each element is a [`TermResult`]: a
/// [`TermResult::Dice`] carries the multiplier, sides, and individual die
/// results, while a [`TermResult::Modifier`] carries the constant value. This
/// keeps the data self-describing — modifiers are not represented as fake
/// single-element "rolls".
///
/// The `total` field contains the net result of evaluating the entire roll expression.
///
/// You can evaluate a roll expression (perform a roll) multiple times by calling
/// [`Roll::rolls`], which returns an iterator of fresh rolls.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Roll {
    /// The original die roll expression that produced this result.
    pub drex: String,
    /// The evaluated result of each term in the expression, in order.
    pub terms: Vec<TermResult>,
    /// The net final result of evaluating all terms in the expression.
    pub total: i64,
}

/// The evaluated result of a single term within a roll expression.
///
/// Returned as the elements of [`Roll::terms`]. A [`Dice`](TermResult::Dice)
/// term records what was rolled; a [`Modifier`](TermResult::Modifier) term
/// records a constant. Use [`TermResult::subtotal`] for the signed contribution
/// of the term to the [`Roll::total`], and [`TermResult::rolls`] for the
/// individual die results (empty for a modifier).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TermResult {
    /// A die-roll term and the individual values it produced.
    Dice {
        /// Number of dice rolled. A negative value subtracts this term's total.
        multiplier: i32,
        /// Number of sides on each die.
        sides: u32,
        /// The individual die results, each in `1..=sides`. Contains
        /// `multiplier.abs()` values.
        rolls: Vec<u32>,
    },
    /// A constant numeric modifier such as `+5` or `-2`.
    Modifier(i32),
}

impl TermResult {
    /// The signed contribution of this term to the [`Roll::total`].
    ///
    /// For a [`Dice`](TermResult::Dice) term this is the sum of its rolls,
    /// negated when the multiplier is negative; for a
    /// [`Modifier`](TermResult::Modifier) it is the modifier value.
    pub fn subtotal(&self) -> i64 {
        match self {
            TermResult::Modifier(n) => *n as i64,
            TermResult::Dice {
                multiplier, rolls, ..
            } => {
                let sum: i64 = rolls.iter().map(|&r| r as i64).sum();
                if *multiplier < 0 { -sum } else { sum }
            }
        }
    }

    /// The individual die results for a [`Dice`](TermResult::Dice) term, or an
    /// empty slice for a [`Modifier`](TermResult::Modifier).
    pub fn rolls(&self) -> &[u32] {
        match self {
            TermResult::Dice { rolls, .. } => rolls,
            TermResult::Modifier(_) => &[],
        }
    }

    /// Recovers the parsed [`DieRollTerm`] that produced this result, so the
    /// term can be re-rolled without re-parsing the expression.
    fn term(&self) -> DieRollTerm {
        match *self {
            TermResult::Dice {
                multiplier, sides, ..
            } => DieRollTerm::DieRoll { multiplier, sides },
            TermResult::Modifier(n) => DieRollTerm::Modifier(n),
        }
    }
}

/// Formats a single evaluated term: `3d6[4, 1, 6]` for dice, `+5` for modifiers.
impl fmt::Display for TermResult {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TermResult::Modifier(n) => write!(f, "{n:+}"),
            TermResult::Dice {
                multiplier,
                sides,
                rolls,
            } => {
                write!(f, "{multiplier}d{sides}{rolls:?}")
            }
        }
    }
}

/// Formats roll results, including die rolls, in a human-readable string.
///
/// For example, if the original expression was `3d6+5`, formatting the `Roll` struct
/// might result in the following text:
///
/// `3d6[3,4,6]+5 (Total: 18)`
impl fmt::Display for Roll {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for term in &self.terms {
            write!(f, "{term}")?;
        }
        write!(f, " (Total: {})", self.total)
    }
}

impl Roll {
    /// Returns an iterator that re-rolls this expression on each call to `next`,
    /// yielding a fresh [`Roll`] every time.
    ///
    /// The iterator is **infinite**, so constrain it with
    /// [`Iterator::take`]. Unlike re-parsing the expression, it reuses the
    /// already-parsed terms, so only the dice are re-rolled.
    ///
    /// ```
    /// let stats: Vec<d20::Roll> = d20::roll_dice("3d6").unwrap().rolls().take(6).collect();
    /// assert_eq!(stats.len(), 6);
    /// ```
    pub fn rolls(&self) -> RollIterator {
        RollIterator {
            drex: self.drex.clone(),
            terms: self.terms.iter().map(TermResult::term).collect(),
        }
    }
}

/// An infinite iterator of fresh rolls, created by [`Roll::rolls`].
pub struct RollIterator {
    drex: String,
    terms: Vec<DieRollTerm>,
}

impl Iterator for RollIterator {
    type Item = Roll;

    fn next(&mut self) -> Option<Roll> {
        let mut rng = rand::rng();
        let terms: Vec<TermResult> = self
            .terms
            .iter()
            .cloned()
            .map(|term| term.evaluate(&mut rng))
            .collect();
        let total = terms.iter().map(TermResult::subtotal).sum();
        Some(Roll {
            drex: self.drex.clone(),
            terms,
            total,
        })
    }
}

/// An individual term parsed from a die roll expression, before it is rolled.
///
/// This is an internal representation; callers receive the evaluated
/// [`TermResult`] instead, via [`Roll::terms`].
#[derive(Debug, Clone, PartialEq, Eq)]
enum DieRollTerm {
    /// A die roll term: roll `multiplier` dice with `sides` sides each.
    DieRoll {
        /// Number of times to roll the given die. A negative value subtracts the
        /// rolled total from the expression. Bounded by [`MAX_DICE`] in magnitude.
        multiplier: i32,
        /// Number of sides on the given die. At least 1 and at most [`MAX_SIDES`].
        sides: u32,
    },
    /// Numeric modifier used in simple left-to-right numeric evaluation of a die roll expression.
    Modifier(i32),
}

impl DieRollTerm {
    /// Parses a single, whitespace-free term such as `3d6`, `-2d10`, `+5`, or `-2`.
    ///
    /// Numbers are parsed into 64-bit integers first so that oversized input is
    /// reported as a descriptive [`D20Error`] instead of panicking.
    fn parse(drt: &str) -> Result<DieRollTerm, D20Error> {
        let lower = drt.to_lowercase();
        if lower.contains('d') {
            let mut parts = lower.splitn(2, 'd');
            let mult_str = parts.next().unwrap_or("");
            let sides_str = parts.next().unwrap_or("");

            // An empty or sign-only count means a single die (e.g. `d6` == `1d6`,
            // `-d6` == `-1d6`).
            let multiplier = match mult_str {
                "" | "+" => 1,
                "-" => -1,
                other => other
                    .parse::<i64>()
                    .map_err(|_| D20Error::InvalidTerm(drt.to_string()))?,
            };
            let sides = sides_str
                .parse::<u64>()
                .map_err(|_| D20Error::InvalidTerm(drt.to_string()))?;

            if multiplier.unsigned_abs() > MAX_DICE as u64 {
                return Err(D20Error::DiceCountTooLarge {
                    count: multiplier.unsigned_abs(),
                    max: MAX_DICE,
                });
            }
            if sides == 0 {
                return Err(D20Error::ZeroSidedDie);
            }
            if sides > MAX_SIDES as u64 {
                return Err(D20Error::SidesTooLarge {
                    sides,
                    max: MAX_SIDES,
                });
            }

            Ok(DieRollTerm::DieRoll {
                multiplier: multiplier as i32,
                sides: sides as u32,
            })
        } else {
            let modifier = drt
                .parse::<i64>()
                .map_err(|_| D20Error::InvalidTerm(drt.to_string()))?;
            if modifier.unsigned_abs() > MAX_MODIFIER as u64 {
                return Err(D20Error::ModifierTooLarge {
                    modifier,
                    max: MAX_MODIFIER,
                });
            }
            Ok(DieRollTerm::Modifier(modifier as i32))
        }
    }

    /// Rolls the dice for this term (or echoes the modifier) using `rng`,
    /// producing the evaluated [`TermResult`].
    fn evaluate<R: Rng + ?Sized>(self, rng: &mut R) -> TermResult {
        match self {
            DieRollTerm::Modifier(n) => TermResult::Modifier(n),
            DieRollTerm::DieRoll { multiplier, sides } => {
                let rolls = (0..multiplier.unsigned_abs())
                    .map(|_| rng.random_range(1..=sides))
                    .collect();
                TermResult::Dice {
                    multiplier,
                    sides,
                    rolls,
                }
            }
        }
    }
}

/// Evaluates the expression string input as a die roll expression (e.g. 3d6 + 4). The
/// results are returned in a `Result` containing either a valid [`Roll`] or a
/// [`D20Error`] describing why the expression could not be evaluated. This function
/// never panics on malformed or out-of-range input.
pub fn roll_dice(s: &str) -> Result<Roll, D20Error> {
    roll_dice_with_rng(s, &mut rand::rng())
}

/// Like [`roll_dice`], but draws randomness from the supplied `rng` instead of
/// the thread-local generator. Pass a seeded RNG (e.g. `StdRng::seed_from_u64`)
/// for deterministic, reproducible rolls.
pub fn roll_dice_with_rng<R: Rng + ?Sized>(s: &str, rng: &mut R) -> Result<Roll, D20Error> {
    let drex = s.trim().to_string();
    let terms = parse_die_roll_terms(&drex)?;

    if terms.is_empty() {
        return Err(D20Error::EmptyExpression);
    }

    let terms: Vec<TermResult> = terms.into_iter().map(|t| t.evaluate(rng)).collect();
    let total = terms.iter().map(TermResult::subtotal).sum();

    Ok(Roll { drex, terms, total })
}

/// Parses a full roll expression into its terms, validating the *entire* input.
///
/// Terms are consumed left to right. The first term may carry an optional sign;
/// every later term must be preceded by an explicit `+`/`-` operator. Any
/// characters that are not part of a valid term produce an error, so unlike a
/// permissive `find_iter` scan this rejects garbage such as `"I have 5 apples"`
/// and ambiguous juxtapositions such as `"2d6 5"` rather than silently
/// extracting numbers from them.
fn parse_die_roll_terms(drex: &str) -> Result<Vec<DieRollTerm>, D20Error> {
    let mut terms = Vec::new();
    let mut pos = 0;

    while pos < drex.len() {
        let rest = &drex[pos..];
        let caps = TERM_RE
            .captures(rest)
            .ok_or_else(|| D20Error::InvalidTerm(rest.to_string()))?;

        let sign = caps.name("sign").map_or("", |m| m.as_str());
        if !terms.is_empty() && sign.is_empty() {
            return Err(D20Error::MissingOperator(rest.to_string()));
        }

        let token = match caps.name("sides") {
            Some(sides) => {
                let count = caps.name("count").map_or("", |m| m.as_str());
                format!("{sign}{count}d{}", sides.as_str())
            }
            None => format!("{sign}{}", caps.name("modval").unwrap().as_str()),
        };
        terms.push(DieRollTerm::parse(&token)?);

        pos += caps.get(0).unwrap().end();
    }

    Ok(terms)
}

/// Generates a random number within the specified inclusive range `[min, max]`.
/// Returns a `Result` containing either the randomly generated `i32` or a
/// [`D20Error::InvalidRange`] when `min > max`.
pub fn roll_range(min: i32, max: i32) -> Result<i32, D20Error> {
    roll_range_with_rng(min, max, &mut rand::rng())
}

/// Like [`roll_range`], but draws randomness from the supplied `rng`.
pub fn roll_range_with_rng<R: Rng + ?Sized>(
    min: i32,
    max: i32,
    rng: &mut R,
) -> Result<i32, D20Error> {
    if min > max {
        Err(D20Error::InvalidRange { min, max })
    } else {
        Ok(rng.random_range(min..=max))
    }
}

#[cfg(test)]
mod tests;
