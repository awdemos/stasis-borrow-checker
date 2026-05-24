# Stasis — A Rust Borrow Checker in Idiomatic Rust

A simplified, educational implementation of a Rust-like borrow checker, written entirely in idiomatic Rust. **Stasis** validates ownership, borrowing, and move semantics for a subset of Rust syntax.

## What it does

Stasis parses a simplified Rust program and runs it through a flow-sensitive borrow checker that enforces:

- **Use after move**: detects reads of moved values
- **Move while borrowed**: prevents moving values with active borrows
- **Multiple mutable borrows**: forbids `&mut x` + `&mut x`
- **Mutable + immutable overlap**: catches conflicts both ways
- **Borrow after move**: can't reference moved values
- **Assign while borrowed**: prevents writes to borrowed variables
- **Block scoping**: borrows and local variables are dropped at `}`
- **Copy types**: `int`, `bool`, `()` are tracked as Copy (no moves); strings are non-Copy

## Example

```rust
fn main() {
    let x = "hello";       // x owns the string
    let r = &x;            // immutable borrow of x
    let y = x;             // ❌ ERROR: can't move x while borrowed
}
```

```rust
fn main() {
    let x = "hello";
    {
        let r = &mut x;    // mutable borrow — OK inside block
    }                       // borrow ends here
    let r2 = &x;            // ✅ OK: mutable borrow was dropped
}
```

## Modules

| Module | Purpose |
|---|---|
| `ast` | AST types for a simplified Rust subset |
| `parser` | Hand-written recursive descent parser |
| `span` | Source location tracking |
| `ownership` | `OwnershipState` machine (Owned → Moved/Borrowed) |
| `borrow_graph` | Active borrow relationship tracking |
| `checker` | Core borrow checking algorithm |
| `error` | Rust-like error diagnostics |

## Grammar

```
program   ::= function*
function  ::= "fn" ident "(" params? ")" ("->" type)? block
block     ::= "{" stmt* expr? "}"
stmt      ::= "let" pat (":" type)? ("=" expr)? ";" | expr ";"
pat       ::= ident | "_"
```

Supports: `if`/`else`, `while`, `return`, `&`/`&mut` references, `.clone()`.

## Run

```bash
cargo run --example demo    # see all rules in action
cargo test                  # 29 tests
```

## Why "Stasis"?

*Stasis* — a state of equilibrium where ownership is static and known at compile time. Also, the opposite of "motion" (mutation, moves).
