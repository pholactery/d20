//! D20
//!
//! # Examples
//! ```
//! extern crate d20;
//!
//! fn main() {
//!     let r = d20::roll_dice("3d6 + 4").unwrap();
//!     assert!(r.total > 6);
//! }
//! ```
extern crate rand;
extern crate regex;

use std::fmt;
use rand::{thread_rng, Rng};
use regex::Regex;



/// The `Roll` struct contains the `DieRollExpression`, the values of
/// the individual rolls, and the calculated total. It is returned by the `roll_dice()`
/// function.
///
/// You can perform a roll mutliple times by converting it into an iterator.
#[derive(Debug)]
pub struct Roll {
    // die roll expression
    pub drex: String,
    // individual die roll results
    pub values: Vec<(DieRollTerm, Vec<i8>)>,
    // result of the drex
    pub total: i32,
}


impl fmt::Display for Roll {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // desired output: 3d6[6,4,3]+5 [Total: 18]
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

/// A `RollIterator` is created when `Roll.into_iter()` method is called.
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

/// `DieRollTerm` represents an indifividual term within a die roll expression.
#[derive(Debug, Clone)]
pub enum DieRollTerm {
    /// `DieRoll1` variant
    DieRoll {
        /// multiplier docs
        multiplier: i8,
        /// sides docs
        sides: u8,
    },
    /// `Modifier` variant
    Modifier(i8),
}


impl DieRollTerm {
    fn parse(drt: &str) -> DieRollTerm {
        if drt.to_lowercase().contains('d') {
            let v: Vec<&str> = drt.split("d").collect();
            DieRollTerm::DieRoll {
                multiplier: v[0].parse::<i8>().unwrap(),
                sides: v[1].parse::<u8>().unwrap(),
            }
        } else {
            DieRollTerm::Modifier(drt.parse::<i8>().unwrap())
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

impl fmt::Display for DieRollTerm {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DieRollTerm::Modifier(n) => write!(f, "{:+}", n),
            DieRollTerm::DieRoll { multiplier: m, sides: s } => write!(f, "{}d{}", m, s),
        }
    }
}

/// `roll_dice()` will evaluate the string input as a die roll expression (e.g. 3d6 + 4).
pub fn roll_dice<'a>(s: &'a str) -> Result<Roll, &'a str> {
    let s: String = s.split_whitespace().collect();
    let terms: Vec<DieRollTerm> = parse_die_roll_terms(&s);

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

fn parse_die_roll_terms(drex: &str) -> Vec<DieRollTerm> {
    let mut terms = Vec::new();

    let re = Regex::new(r"([+-]?\s*\d+[dD]\d+|[+-]?\s*\d+)").unwrap();

    let matches = re.find_iter(drex);
    for m in matches {
        println!("{:?}", m);
        terms.push(DieRollTerm::parse(&drex[m.start()..m.end()]));
    }
    terms
}

/// `roll_range()` will generate a random number within the specified range and return that value.
pub fn roll_range<'a>(min: i32, max: i32) -> Result<i32, &'a str> {
    if min > max {
        Err("Invalid range: min must be less than or equal to max")
    } else {
        Ok(thread_rng().gen_range(min, max + 1))
    }
}

#[cfg(test)]
mod tests;
