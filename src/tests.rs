use crate::DieRollTerm;
use crate::{Roll, TermResult};
use crate::{parse_die_roll_terms, roll_dice, roll_range};

#[test]
fn die_roll_expression_parsed() {
    //NOTE: assumes properly trimmed die roll expression
    let pd = "3d12+4".to_string();
    let nd = "-4d10+5".to_string();
    let mf = "50+2d8-1d4".to_string();

    let pv = parse_die_roll_terms(&pd).unwrap();
    if let DieRollTerm::DieRoll {
        multiplier: m,
        sides: s,
    } = pv[0]
    {
        assert_eq!(m, 3);
        assert_eq!(s, 12);
    }
    if let DieRollTerm::Modifier(n) = pv[1] {
        assert_eq!(n, 4);
    }

    let nv = parse_die_roll_terms(&nd).unwrap();
    if let DieRollTerm::DieRoll {
        multiplier: m,
        sides: s,
    } = nv[0]
    {
        assert_eq!(m, -4);
        assert_eq!(s, 10);
    }
    if let DieRollTerm::Modifier(n) = nv[1] {
        assert_eq!(n, 5);
    }

    let mv = parse_die_roll_terms(&mf).unwrap();
    if let DieRollTerm::Modifier(n) = mv[0] {
        assert_eq!(n, 50);
    }
    if let DieRollTerm::DieRoll {
        multiplier: m,
        sides: s,
    } = mv[1]
    {
        assert_eq!(m, 2);
        assert_eq!(s, 8);
    }
    if let DieRollTerm::DieRoll {
        multiplier: m,
        sides: s,
    } = mv[2]
    {
        assert_eq!(m, -1);
        assert_eq!(s, 4);
    }
}

#[test]
fn die_roll_term_parsed() {
    let drt = "3d6".to_string();
    let mfy = "+7".to_string();
    let drt = DieRollTerm::parse(&drt).unwrap();
    let mfy = DieRollTerm::parse(&mfy).unwrap();
    if let DieRollTerm::DieRoll {
        multiplier: m,
        sides: s,
    } = drt
    {
        assert_eq!(m, 3);
        assert_eq!(s, 6);
    } else {
        panic!("expected a DieRoll term");
    }

    if let DieRollTerm::Modifier(n) = mfy {
        assert_eq!(n, 7);
    } else {
        panic!("expected a Modifier term");
    }
}

#[test]
fn die_roll_term_calculated() {
    let mut rng = rand::rng();
    let dt = DieRollTerm::parse("6d1").unwrap().evaluate(&mut rng);
    let nt = DieRollTerm::parse("-4d1").unwrap().evaluate(&mut rng);
    let pm = DieRollTerm::parse("+7").unwrap().evaluate(&mut rng);
    let nm = DieRollTerm::parse("-7").unwrap().evaluate(&mut rng);

    assert_eq!(dt.subtotal(), 6);
    assert_eq!(nt.subtotal(), -4);
    assert_eq!(pm.subtotal(), 7);
    assert_eq!(nm.subtotal(), -7);
}

#[test]
fn die_roll_term_evaluated() {
    let drt = DieRollTerm::parse("3d1").unwrap();
    let v = drt.evaluate(&mut rand::rng());

    assert_eq!(v.rolls().len(), 3);
    assert_eq!(v.rolls(), [1, 1, 1]);
}

#[test]
fn die_roll_term_modifier_evaluated() {
    let mfy = DieRollTerm::parse("+7").unwrap();
    let mfy2 = DieRollTerm::parse("-7").unwrap();
    let mut rng = rand::rng();
    let v1 = mfy.evaluate(&mut rng);
    let v2 = mfy2.evaluate(&mut rng);

    // A modifier has no individual rolls; its value is its subtotal.
    assert!(v1.rolls().is_empty());
    assert!(v2.rolls().is_empty());
    assert_eq!(v1.subtotal(), 7);
    assert_eq!(v2.subtotal(), -7);
}

#[test]
fn roll_dice_produces_roll_for_valid_expression() {
    let s = "2d6 + 6 + 4d10";
    let r = roll_dice(s);
    let r = r.unwrap();

    // drex now preserves the original expression (C12), not a stripped form.
    assert_eq!(r.drex, "2d6 + 6 + 4d10".to_string());
    assert_eq!(r.terms.len(), 3);
    assert_eq!(r.terms[0].rolls().len(), 2); // two d6 rolls
    assert_eq!(r.terms[1].rolls().len(), 0); // a modifier has no rolls
    assert_eq!(r.terms[2].rolls().len(), 4); // four d10 rolls

    let s = "3d1 + 2d1 + 1";
    let r = roll_dice(s);
    let r = r.unwrap();
    assert_eq!(r.total, 6);

    let s = "-3d1 + 2d1 + 1";
    let r = roll_dice(s);
    let r = r.unwrap();
    assert_eq!(r.total, 0);
}

#[test]
fn roll_dice_produces_error_for_invalid_expression() {
    let s = "two plus two equals CHICKEN!";
    assert!(roll_dice(s).is_err());
}

#[test]
fn result_range_roll_produces_result_in_range() {
    let r1 = roll_range(3, 3);
    let r1 = r1.unwrap();
    let r2 = roll_range(4, 4);
    let r2 = r2.unwrap();

    assert_eq!(r1, 3);
    assert_eq!(r2, 4);
}

#[test]
fn roll_range_min_max_switched() {
    assert!(roll_range(12, 1).is_err());
}

#[test]
fn iterator_yields_new_results() {
    let r = roll_dice("3d6");
    let v: Vec<Roll> = r.unwrap().rolls().take(6).collect();

    assert_eq!(v.len(), 6);
    assert!(v[0].total >= 3 && v[0].total <= 18);
}

#[test]
fn term_result_displays_properly() {
    let dice = TermResult::Dice {
        multiplier: 3,
        sides: 6,
        rolls: vec![4, 1, 6],
    };
    assert_eq!(format!("{dice}"), "3d6[4, 1, 6]");
    assert_eq!(format!("{}", TermResult::Modifier(5)), "+5");
    assert_eq!(format!("{}", TermResult::Modifier(-6)), "-6");
}

#[test]
fn roll_displays_properly() {
    let roll = roll_dice("3d1 + 5").unwrap();
    let bigger_roll = roll_dice("3d1 - 2d1 - 4").unwrap();

    let out = format!("{}", roll);
    assert_eq!(out, "3d1[1, 1, 1]+5 (Total: 8)");

    let out = format!("{}", bigger_roll);
    assert_eq!(out, "3d1[1, 1, 1]-2d1[1, 1]-4 (Total: -3)");
}
