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
//! fn main() {
//!     let r = d20::roll_dice("3d6 + 4").unwrap();
//!     assert!(r.total > 6);
//!     let r = d20::roll_dice("1d1-3").unwrap();
//!     assert_eq!(r.total, -2);
//!
//!     let r = d20::roll_dice("roll four chickens and add six ferrets");
//!     match r {
//!        Ok(_) => assert!(false), // this should NOT be ok, fail
//!        Err(_) => assert!(true), // bad expressions produce errors
//!    }
//! }
//! ```
//! ### Iterating Roll
//! A valid `Roll` can be converted into an open ended iterator via its `into_iter()` method, providing successive
//! rolls of the given die roll expression.
//!
//! _Note that it will be necessary to constrain the iterator via `take(n)`._
//! 
//! ```rust
//! use d20::*;
//!
//! fn main() {
//!     let v: Vec<Roll> = d20::roll_dice("3d6").unwrap().into_iter().take(3).collect();
//!
//!     assert_eq!(v.len(), 3);
//!     assert!(v[0].total >= 3 && v[0].total <= 18);
//!     assert!(v[1].total >= 3 && v[1].total <= 18);
//!     assert!(v[2].total >= 3 && v[2].total <= 18);     
//! }
//!
//! ```
//!
//! ### Range Rolls
//! If you are less concerned about dice rolls and require only a random number within a given range, `roll_range()`
//! will do just that.
//!
//! ```rust
//! # fn main() {
//!     let rg = d20::roll_range(1,100).unwrap();
//!     assert!(rg >= 1 && rg <= 100);
//! # }
//! ```
//!
//!
use std::fmt;
use std::sync::LazyLock;
use rand::RngExt;
use regex::Regex;

/// Compiled once on first use and reused for every parse. The pattern matches a
/// signed die-roll term (`3d6`, `-2d10`) or a signed numeric modifier (`+5`, `-2`).
static DICE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"([+-]?\s*\d+[dD]\d+|[+-]?\s*\d+)").unwrap());

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
    /// A term could not be parsed (e.g. a number too large to fit a 64-bit integer).
    #[error("invalid term: '{0}'")]
    InvalidTerm(String),
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
/// The `Roll` struct contains the original _die roll expression_ passed to the `roll_dice()`
/// function.
///
/// The list of `values` will always be a vector containing at least one element because roll 
/// expressions are not valid without at least 1 term. Each resulting value is a tuple containing
/// the parsed `DieRollTerm` and a vector of values. For `DieRollTerm::Modifier` terms, this will be a single-element
/// vector containing the modifier value. For `DieRollTerm::DieRoll` terms, this will be a vector
/// containing the results of each die roll.
///
/// The `total` field contains the net result of evaluating the entire roll expression.
///
/// You can evaluate a roll expression (perform a roll) mutliple times by converting it into an iterator.
#[derive(Debug)]
pub struct Roll {
    /// A die roll expression conforming to the format specification
    pub drex: String,
    /// The results of evaluating each term in the expression
    pub values: Vec<(DieRollTerm, Vec<i32>)>,
    /// The net final result of evaluating all terms in the expression
    pub total: i64,
}


/// Formats roll results, including die rolls, in a human-readable string. 
///
/// For example, if the original expression was `3d6+5`, formatting the `Roll` struct
/// might result in the following text:
///
/// `3d6[3,4,6]+5 (Total: 18)`
impl fmt::Display for Roll {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {        
        let mut out = String::new();

        for i in 0..self.values.len() {
            let ref val = self.values[i];
            match val.0 {
                DieRollTerm::Modifier(_) => out = out + format!("{}", &val.0).as_str(),
                DieRollTerm::DieRoll { .. } => {
                    out = out + format!("{}{:?}", &val.0, val.1).as_str();
                }
            };
        }
        out = format!("{} (Total: {})", out, self.total);
        write!(f, "{}", out)
    }
}

/// Converts an evaluated roll expression into an iterator, allowing the expression
/// to be evaluated (including re-rolling of dice) multiple times. 
impl IntoIterator for Roll {
    type Item = Roll;
    type IntoIter = RollIterator;

    fn into_iter(self) -> Self::IntoIter {
        RollIterator {
            roll: self,
            index: 0,
        }
    }
}

/// A `RollIterator` is created when `into_iter()` is called on a `Roll`.
pub struct RollIterator {
    roll: Roll,
    index: usize,
}

impl Iterator for RollIterator {
    type Item = Roll;

    fn next(&mut self) -> Option<Roll> {
        let result = roll_dice(&self.roll.drex);
        match result {
            Ok(r) => {
                self.index += 1;
                Some(r)
            }
            Err(_) => return None,
        }
    }
}

/// Represents an individual term within a die roll expression. Terms can either be numeric
/// modifiers like `+5` or `-2` or they can be terms indicating die rolls.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DieRollTerm {
    /// Indicates a die roll term to roll `multiplier` dice with `sides` sides.
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

            let multiplier = mult_str
                .parse::<i64>()
                .map_err(|_| D20Error::InvalidTerm(drt.to_string()))?;
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
                return Err(D20Error::SidesTooLarge { sides, max: MAX_SIDES });
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
                return Err(D20Error::ModifierTooLarge { modifier, max: MAX_MODIFIER });
            }
            Ok(DieRollTerm::Modifier(modifier as i32))
        }
    }

    /// Computes the signed contribution of an already-evaluated term to the total.
    fn calculate(v: &(DieRollTerm, Vec<i32>)) -> i64 {
        match v.0 {
            DieRollTerm::Modifier(n) => n as i64,
            DieRollTerm::DieRoll { multiplier, .. } => {
                let sum: i64 = v.1.iter().map(|&val| val as i64).sum();
                if multiplier < 0 { -sum } else { sum }
            }
        }
    }

    /// Rolls the dice for this term (or echoes the modifier), returning the term
    /// alongside the individual values produced.
    fn evaluate(self) -> (DieRollTerm, Vec<i32>) {
        match self {
            DieRollTerm::Modifier(n) => (self, vec![n]),
            DieRollTerm::DieRoll { multiplier, sides } => {
                let mut rng = rand::rng();
                let rolls = (0..multiplier.unsigned_abs())
                    .map(|_| rng.random_range(1..=sides) as i32)
                    .collect();
                (self, rolls)
            }
        }
    }
}

/// Formats an individual die roll term in a human-friendly fashion. For `Modifier` terms,
/// this will force the printing of a + or - sign before the modifier value. For `DieRoll`
/// terms, this displays the term in the form `5d10`. 
impl fmt::Display for DieRollTerm {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DieRollTerm::Modifier(n) => write!(f, "{:+}", n),
            DieRollTerm::DieRoll { multiplier: m, sides: s } => write!(f, "{}d{}", m, s),
        }
    }
}

/// Evaluates the expression string input as a die roll expression (e.g. 3d6 + 4). The
/// results are returned in a `Result` containing either a valid [`Roll`] or a
/// [`D20Error`] describing why the expression could not be evaluated. This function
/// never panics on malformed or out-of-range input.
pub fn roll_dice(s: &str) -> Result<Roll, D20Error> {
    let s: String = s.split_whitespace().collect();
    let terms = parse_die_roll_terms(&s)?;

    if terms.is_empty() {
        return Err(D20Error::EmptyExpression);
    }

    let values: Vec<(DieRollTerm, Vec<i32>)> =
        terms.into_iter().map(DieRollTerm::evaluate).collect();
    let total = values.iter().map(DieRollTerm::calculate).sum();

    Ok(Roll { drex: s, values, total })
}

fn parse_die_roll_terms(drex: &str) -> Result<Vec<DieRollTerm>, D20Error> {
    DICE_RE
        .find_iter(drex)
        .map(|m| DieRollTerm::parse(m.as_str()))
        .collect()
}

/// Generates a random number within the specified inclusive range `[min, max]`.
/// Returns a `Result` containing either the randomly generated `i32` or a
/// [`D20Error::InvalidRange`] when `min > max`.
pub fn roll_range(min: i32, max: i32) -> Result<i32, D20Error> {
    if min > max {
        Err(D20Error::InvalidRange { min, max })
    } else {
        Ok(rand::rng().random_range(min..=max))
    }
}

#[cfg(test)]
mod tests;
