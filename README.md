# d20

[![Crates.io](https://img.shields.io/crates/v/d20.svg)](https://crates.io/crates/d20) [![docs.rs](https://docs.rs/d20/badge.svg)](https://docs.rs/d20/)


**D20** is a simple crate designed to evaluate _roll expressions_. A _roll expression_ is an
english-language string that reflects the intent of a dungeon or game master to perform a 
particular roll.

For example, in a tabletop game you may frequently hear phrases like _"roll 2d10"_, or 
_"roll 3d6 and add 5"_. These are roll expressions, and the components within them are
what we call _die roll terms_. A _die roll term_ is either a term that calls for the rolling
of an n-sided die x times (e.g. 3d6) or a modifier that simply adds or subtracts a constant value
from the larger expression.

Examples of valid _roll expressions_ include:

* 3d6
* 2d10 + 5
* 1d20-3
* +6
* -2
* 3d10+5d100-21+7

Roll expressions can have arbitrary length and complexity, and it is perfectly legal for the final result
of a roll expression to be negative after applying modifiers.

# Examples
```
extern crate d20;

fn main() {
    let r = d20::roll_dice("3d6 + 4").unwrap();
    assert!(r.total > 6);
    let r = d20::roll_dice("1d1-3").unwrap();
    assert_eq!(r.total, -2);

    let r = d20::roll_dice("roll four chickens and add six ferrets");
    match r {
       Ok(_) => assert!(false), // this should NOT be ok, fail
       Err(_) => assert!(true), // bad expressions produce errors
   }
}
```
### Iterating Roll
A valid `Roll` can be converted into an open ended iterator via its `into_iter()` method, providing successive
rolls of the given die roll expression.

_Note that it will be necessary to constrain the iterator via `take(n)`._
 
```rust
extern crate d20;
use d20::*;

fn main() {
    let raw_stats: Vec<Roll> = d20::roll_dice("3d6").unwrap().into_iter().take(6).collect();

    println!("\nCHARACTER STATS:");
    println!("  STR: {}", raw_stats[0].total);
    println!("  INT: {}", raw_stats[1].total);
    println!("  WIS: {}", raw_stats[2].total);
    println!("  DEX: {}", raw_stats[3].total);
    println!("  CON: {}", raw_stats[4].total);
    println!("  CHA: {}", raw_stats[5].total);
}
 ```

### Range Rolls
If you are less concerned about dice rolls and require only a random number within a given range, `roll_range()`
will do just that.

```rust
extern crate d20;
fn main() {
    let rg = d20::roll_range(1,100).unwrap();
    assert!(rg >= 1 && rg <= 100);
}
```