use borrow_checker::BorrowChecker;
use borrow_checker::parser::Parser;

fn check(code: &str) -> Result<(), borrow_checker::error::BorrowError> {
    let mut parser = Parser::new(code);
    let program = parser.parse()?;
    let mut checker = BorrowChecker::new();
    checker.check_program(&program)
}

fn expect_ok(code: &str) {
    if let Err(e) = check(code) {
        panic!("Expected OK but got error: {:?}\nCode:\n{}", e, code);
    }
}

fn expect_err(code: &str) -> borrow_checker::error::BorrowError {
    match check(code) {
        Ok(()) => panic!("Expected error but got OK\nCode:\n{}", code),
        Err(e) => e,
    }
}

// ── Parsing ───────────────────────────────────────────────────────────

#[test]
fn parse_empty_program() {
    expect_ok("");
}

#[test]
fn parse_simple_function() {
    expect_ok("fn main() {}");
}

#[test]
fn parse_function_with_params() {
    expect_ok("fn add(x: int, y: int) -> int { x + y }");
}

#[test]
fn parse_if_else() {
    expect_ok("fn main() { if true { 1 } else { 2 } }");
}

#[test]
fn parse_while() {
    expect_ok("fn main() { while true { 0 } }");
}

#[test]
fn parse_return() {
    expect_ok("fn main() -> int { return 42 }");
}

#[test]
fn parse_let_stmt() {
    expect_ok("fn main() { let x: int = 10; }");
}

#[test]
fn parse_let_infer() {
    expect_ok("fn main() { let x = 10; }");
}

#[test]
fn parse_block_expr() {
    expect_ok("fn main() -> int { let x = { let y = 1; y + 1 }; x }");
}

// ── Copy types (int, bool) ────────────────────────────────────────────

#[test]
fn int_does_not_move() {
    expect_ok(
        "
        fn main() {
            let x = 42;
            let y = x;
            let z = x;
        }
    ",
    );
}

#[test]
fn bool_does_not_move() {
    expect_ok(
        "
        fn main() {
            let x = true;
            let y = x;
            let z = x;
        }
    ",
    );
}

// ── Move semantics ────────────────────────────────────────────────────

#[test]
fn use_after_move_is_error() {
    let err = expect_err(
        "
        fn main() {
            let x = \"hello\";
            let y = x;
            let z = x;
        }
    ",
    );
    assert!(
        matches!(err, borrow_checker::error::BorrowError::UseAfterMove { .. }),
        "Expected UseAfterMove error, got: {:?}",
        err
    );
}

#[test]
fn assign_after_move_is_error() {
    let err = expect_err(
        "
        fn main() {
            let x = \"hello\";
            let y = x;
            x = \"world\";
        }
    ",
    );
    assert!(
        matches!(err, borrow_checker::error::BorrowError::UseAfterMove { .. }),
        "Expected UseAfterMove error, got: {:?}",
        err
    );
}

// ── Borrowing ─────────────────────────────────────────────────────────

#[test]
fn immutable_borrow_ok() {
    expect_ok(
        "
        fn main() {
            let x = \"hello\";
            let r = &x;
        }
    ",
    );
}

#[test]
fn multiple_immutable_borrows_ok() {
    expect_ok(
        "
        fn main() {
            let x = \"hello\";
            let r1 = &x;
            let r2 = &x;
        }
    ",
    );
}

#[test]
fn mutable_borrow_ok() {
    expect_ok(
        "
        fn main() {
            let x = \"hello\";
            let r = &mut x;
        }
    ",
    );
}

#[test]
fn multiple_mutable_borrows_is_error() {
    let err = expect_err(
        "
        fn main() {
            let x = \"hello\";
            let r1 = &mut x;
            let r2 = &mut x;
        }
    ",
    );
    assert!(
        matches!(
            err,
            borrow_checker::error::BorrowError::MultipleMutableBorrows { .. }
        ),
        "Expected MultipleMutableBorrows, got: {:?}",
        err
    );
}

#[test]
fn mutable_after_immutable_is_error() {
    let err = expect_err(
        "
        fn main() {
            let x = \"hello\";
            let r1 = &x;
            let r2 = &mut x;
        }
    ",
    );
    assert!(
        matches!(
            err,
            borrow_checker::error::BorrowError::MutableBorrowWhileImmutable { .. }
        ),
        "Expected MutableBorrowWhileImmutable, got: {:?}",
        err
    );
}

#[test]
fn immutable_after_mutable_is_error() {
    let err = expect_err(
        "
        fn main() {
            let x = \"hello\";
            let r1 = &mut x;
            let r2 = &x;
        }
    ",
    );
    assert!(
        matches!(
            err,
            borrow_checker::error::BorrowError::ImmutableBorrowWhileMutable { .. }
        ),
        "Expected ImmutableBorrowWhileMutable, got: {:?}",
        err
    );
}

// ── Blocks ────────────────────────────────────────────────────────────

#[test]
fn block_scope_drops_locals() {
    expect_ok(
        "
        fn main() {
            let x = \"hello\";
            {
                let y = &x;
            }
            let z = &x;
        }
    ",
    );
}

#[test]
fn borrow_ends_at_block_end() {
    expect_ok(
        "
        fn main() {
            let x = \"hello\";
            {
                let r = &mut x;
            }
            let r2 = &x;
        }
    ",
    );
}

// ── Move while borrowed ───────────────────────────────────────────────

#[test]
fn move_while_borrowed_is_error() {
    let err = expect_err(
        "
        fn main() {
            let x = \"hello\";
            let r = &x;
            let y = x;
        }
    ",
    );
    assert!(
        matches!(
            err,
            borrow_checker::error::BorrowError::MoveWhileBorrowed { .. }
        ),
        "Expected MoveWhileBorrowed, got: {:?}",
        err
    );
}

#[test]
fn assign_while_borrowed_is_error() {
    let err = expect_err(
        "
        fn main() {
            let x = \"hello\";
            let r = &x;
            x = \"world\";
        }
    ",
    );
    assert!(
        matches!(
            err,
            borrow_checker::error::BorrowError::AssignWhileBorrowed { .. }
        ),
        "Expected AssignWhileBorrowed, got: {:?}",
        err
    );
}

// ── Clone ─────────────────────────────────────────────────────────────

#[test]
fn clone_does_not_move() {
    expect_ok(
        "
        fn main() {
            let x = \"hello\";
            let y = x.clone();
            let z = x;
        }
    ",
    );
}

// ── Variable not found ────────────────────────────────────────────────

#[test]
fn undefined_variable_is_error() {
    let err = expect_err(
        "
        fn main() {
            let y = x;
        }
    ",
    );
    assert!(
        matches!(
            err,
            borrow_checker::error::BorrowError::VariableNotFound { .. }
        ),
        "Expected VariableNotFound, got: {:?}",
        err
    );
}

// ── Borrow after move ─────────────────────────────────────────────────

#[test]
fn borrow_after_move_is_error() {
    let err = expect_err(
        "
        fn main() {
            let x = \"hello\";
            let y = x;
            let r = &x;
        }
    ",
    );
    assert!(
        matches!(
            err,
            borrow_checker::error::BorrowError::BorrowAfterMove { .. }
        ) || matches!(err, borrow_checker::error::BorrowError::UseAfterMove { .. }),
        "Expected BorrowAfterMove or UseAfterMove, got: {:?}",
        err
    );
}

// ── Programs that should work ─────────────────────────────────────────

#[test]
fn valid_complex_program() {
    expect_ok(
        "
        fn main() {
            let greeting = \"hello\";
            let r = &greeting;
            let len = 5;
            if len > 0 {
                let msg = r;
            } else {
                let msg = greeting.clone();
            }
        }
    ",
    );
}

#[test]
fn valid_borrow_reborrow() {
    expect_ok(
        "
        fn main() {
            let x = \"hello\";
            let r1 = &x;
            let r2 = &*r1;
        }
    ",
    );
}

#[test]
fn valid_mut_borrow_ends_before_read() {
    expect_ok(
        "
        fn main() {
            let x = \"hello\";
            {
                let r = &mut x;
            }
            let s = x;
        }
    ",
    );
}
