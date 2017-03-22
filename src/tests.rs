use Roll;
use DieRollTerm;
use {roll_dice, roll_range, parse_die_roll_terms};

#[test]
fn die_roll_expression_parsed() {
    //NOTE: assumes properly trimmed die roll expression
    let pd = "3d12+4".to_string();
    let nd = "-4d10+5".to_string();
    let mf = "50+2d8-1d4".to_string();

    let pv = parse_die_roll_terms(&pd);
    if let DieRollTerm::DieRoll { multiplier: m, sides: s } = pv[0] {
        assert_eq!(m, 3);
        assert_eq!(s, 12);
    }
    if let DieRollTerm::Modifier(n) = pv[1] {
        assert_eq!(n, 4);
    }

    let nv = parse_die_roll_terms(&nd);
    if let DieRollTerm::DieRoll { multiplier: m, sides: s } = nv[0] {
        assert_eq!(m, -4);
        assert_eq!(s, 10);
    }
    if let DieRollTerm::Modifier(n) = nv[1] {
        assert_eq!(n, 5);
    }

    let mv = parse_die_roll_terms(&mf);
    if let DieRollTerm::Modifier(n) = mv[0] {
        assert_eq!(n, 50);
    }
    if let DieRollTerm::DieRoll { multiplier: m, sides: s } = mv[1] {
        assert_eq!(m, 2);
        assert_eq!(s, 8);
    }
    if let DieRollTerm::DieRoll { multiplier: m, sides: s } = mv[2] {
        assert_eq!(m, -1);
        assert_eq!(s, 4);
    }

}

#[test]
fn die_roll_term_parsed() {
    let drt = "3d6".to_string();
    let mfy = "+7".to_string();
    let drt = DieRollTerm::parse(&drt);
    let mfy = DieRollTerm::parse(&mfy);
    if let DieRollTerm::DieRoll { multiplier: m, sides: s } = drt {
        assert_eq!(m, 3);
        assert_eq!(s, 6);
    } else {
        assert!(false);
    }

    if let DieRollTerm::Modifier(n) = mfy {
        assert_eq!(n, 7);
    } else {
        assert!(false);
    }
}

#[test]
fn die_roll_term_calculated() {
    let dt = DieRollTerm::parse("6d1").evaluate();
    let nt = DieRollTerm::parse("-4d1").evaluate();
    let pm = DieRollTerm::parse("+7").evaluate();
    let nm = DieRollTerm::parse("-7").evaluate();
    let rng = DieRollTerm::parse("3d10").evaluate();

    let dtr = DieRollTerm::calculate(dt);
    assert_eq!(dtr, 6);

    let ntr = DieRollTerm::calculate(nt);
    assert_eq!(ntr, -4);

    let pmr = DieRollTerm::calculate(pm);
    assert_eq!(pmr, 7);

    let nmr = DieRollTerm::calculate(nm);
    assert_eq!(nmr, -7);

    //let rngr = DieRollTerm::calculate(rng);
    //assert!((3..31).contains(rngr));
}

#[test]
fn die_roll_term_evaluated() {
    let drt = DieRollTerm::parse("3d1");
    let v = drt.evaluate();

    assert_eq!(v.1.len(), 3);
    assert_eq!(v.1[0], 1);
    assert_eq!(v.1[1], 1);
    assert_eq!(v.1[2], 1);
}

#[test]
fn die_roll_term_modifier_evaluated() {
    let mfy = DieRollTerm::parse("+7");
    let mfy2 = DieRollTerm::parse("-7");
    let v1 = mfy.evaluate();
    let v2 = mfy2.evaluate();

    assert_eq!(v1.1.len(), 1);
    assert_eq!(v2.1.len(), 1);
    assert_eq!(v1.1[0], 7);
    assert_eq!(v2.1[0], -7);
}

#[test]
fn roll_dice_produces_roll_for_valid_expression() {
    let s = "2d6 + 6 + 4d10".to_string();
    let r = roll_dice(s);
    let r = r.unwrap();

    assert_eq!(r.drex, "2d6+6+4d10".to_string());
    assert_eq!(r.values.len(), 3);
    assert_eq!(r.values[0].1.len(), 2);
    assert_eq!(r.values[1].1.len(), 1);
    assert_eq!(r.values[2].1.len(), 4);

    let s = "3d1 + 2d1 + 1".to_string();
    let r = roll_dice(s);
    let r = r.unwrap();
    assert_eq!(r.total, 6);

    let s = "-3d1 + 2d1 + 1".to_string();
    let r = roll_dice(s);
    let r = r.unwrap();
    assert_eq!(r.total, 0);
}

#[test]
fn roll_dice_produces_error_for_invalid_expression() {
    let s = "two plus two equals CHICKEN!".to_string();
    let r = roll_dice(s);

    match r {
        Ok(_) => assert!(false),
        Err(_) => assert!(true),
    }
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
    let r = roll_range(12, 1);

    match r {
        Ok(_) => assert!(false),
        Err(_) => assert!(true),
    }
}
