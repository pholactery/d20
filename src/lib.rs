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
//! extern crate d20;
//!
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
//! extern crate d20;
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
//! # extern crate d20;
//! # fn main() {
//!     let rg = d20::roll_range(1,100).unwrap();
//!     assert!(rg >= 1 && rg <= 100);
//! # }
//! ```
//!
//! 
extern crate rand;
extern crate regex;

use std::{fmt, error::Error};
use rand::{thread_rng, Rng};
use regex::Regex;



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
    pub values: Vec<(DieRollTerm, Vec<i8>)>,
    /// The net final result of evaluating all terms in the expression
    pub total: i32,
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
#[derive(Debug, Clone)]
pub enum DieRollTerm {
    /// Indicates a die roll term to roll `multiplier` dice with `sides` sides.
    DieRoll {
        /// Number of times to roll the given die
        multiplier: i8,
        /// Number of sides on the given die
        sides: u8,
    },
    /// Numeric modifier used in simple left-to-right numeric evaluation of a die roll expression.
    Modifier(i8),
}


impl DieRollTerm {
    fn parse(drt: &str) -> Result<DieRollTerm, Box<dyn Error>> {
        if drt.to_lowercase().contains('d') {
            let v: Vec<&str> = drt.split("d").collect();
            Ok(DieRollTerm::DieRoll {
                multiplier: v[0].parse::<i8>()?,
                sides: v[1].parse::<u8>()?,
            })
        } else {
            Ok(DieRollTerm::Modifier(drt.parse::<i8>()?))
        }
    }


    fn calculate(v: (DieRollTerm, Vec<i8>)) -> i32 {
        match v.0 {
            DieRollTerm::Modifier(n) => n as i32,
            DieRollTerm::DieRoll { multiplier: m, .. } => {
                let mut sum: i32 = v.1.iter().fold(0i32, |sum, &val| sum + val as i32);
                if m < 0 {
                    sum = sum * -1;
                }
                sum
            }
        }
    }

    fn evaluate(self) -> (DieRollTerm, Vec<i8>) {
        match self {
            DieRollTerm::Modifier(n) => (self, vec![n]),
            DieRollTerm::DieRoll { multiplier: m, sides: s } => {
                (self, (0..m.abs()).map(|_| thread_rng().gen_range(1, s as i8 + 1)).collect())
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
/// results are returned in a `Result` object that contains either a valid `Roll` or some
/// text indicating why the function was unable to roll the dice / evaluate the expression.
pub fn roll_dice<'a>(s: &'a str) -> Result<Roll, &'a str> {
    let s: String = s.split_whitespace().collect();
    let terms: Vec<DieRollTerm> = match parse_die_roll_terms(&s) {
        Ok(t) => t,
        Err(_) => return Err("Invalid die roll expression: unable to parse terms."),
    };

    if terms.len() == 0 {
        Err("Invalid die roll expression: no die roll terms found.")
    } else {

        let v: Vec<_> = terms.into_iter().map(|t| t.evaluate()).collect();
        let t = v.clone();

        Ok(Roll {
            drex: s,
            values: v,
            total: t.into_iter().fold(0i32, |sum, val| sum + DieRollTerm::calculate(val)),
        })
    }
}

fn parse_die_roll_terms(drex: &str) -> Result<Vec<DieRollTerm>, Box<dyn Error>> {
    let mut terms = Vec::new();

    let re = Regex::new(r"([+-]?\s*\d+[dD]\d+|[+-]?\s*\d+)")?;

    let matches = re.find_iter(drex);
    for m in matches {
        terms.push(DieRollTerm::parse(&drex[m.start()..m.end()])?);
    }
    Ok(terms)
}

/// Generates a random number within the specified range. Returns a `Result` containing
/// either a valid signed 32-bit integer with the randomly generated number or some text 
/// indicating the reason for failure.
pub fn roll_range<'a>(min: i32, max: i32) -> Result<i32, &'a str> {
    if min > max {
        Err("Invalid range: min must be less than or equal to max")
    } else {
        Ok(thread_rng().gen_range(min, max + 1))
    }
}

#[cfg(test)]
mod tests;
