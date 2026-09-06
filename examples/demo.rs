use borrow_checker::BorrowChecker;

fn check(name: &str, code: &str) {
    println!("─── {name} ───");
    let mut checker = BorrowChecker::new();
    match checker.check_source(code) {
        Ok(()) => println!("  ✅ ACCEPTED\n"),
        Err(e) => {
            println!("{}", e.format_with_source(code));
            println!();
        }
    }
}

fn main() {
    check(
        "Copy types (int)",
        "
        fn main() {
            let x = 42;
            let y = x;
            let z = x;
        }
    ",
    );

    check(
        "Immutable borrow",
        "
        fn main() {
            let x = \"hello\";
            let r1 = &x;
            let r2 = &x;
        }
    ",
    );

    check(
        "Mutable borrow",
        "
        fn main() {
            let x = \"hello\";
            let r = &mut x;
        }
    ",
    );

    check(
        "Block scoping drops borrows",
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

    check(
        "Clone avoids move",
        "
        fn main() {
            let x = \"hello\";
            let y = x.clone();
            let z = x;
        }
    ",
    );

    check(
        "Use after move",
        "
        fn main() {
            let x = \"hello\";
            let y = x;
            let z = x;
        }
    ",
    );

    check(
        "Move while borrowed",
        "
        fn main() {
            let x = \"hello\";
            let r = &x;
            let y = x;
        }
    ",
    );

    check(
        "Mutable + immutable conflict",
        "
        fn main() {
            let x = \"hello\";
            let r1 = &x;
            let r2 = &mut x;
        }
    ",
    );
}
