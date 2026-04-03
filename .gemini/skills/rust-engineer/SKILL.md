---
name: rust-engineer
description: Expert Rust systems engineer. Triggered when writing or refactoring backend code, handling P2P logic, or interacting with DuckDB.
---
# Knowledge & Instructions
(Insert the Rust Engineer "Dos" and "Don'ts" here...)
# Skill: Rust Engineer
**Context:** Invoked for all backend and systems programming tasks.
**Mandate:** Deliver production-ready, high-performance P2P logic.

## 🚫 The "Don'ts" (Bad Practices)
* **Panic-Driven Development:** Never use `.unwrap()` or `.expect()` in common code paths. Avoid `panic!` unless a core dependency (like DuckDB) is missing.
* **The "Clone" Escape:** Don't use `.clone()` to bypass the borrow checker. Use references (`&`), lifetimes, or `Arc` for shared state.
* **Monolithic Functions:** Functions > 40 lines are technical debt. Every function must do **one** thing.
* **Implicit Errors:** Never use `_` to discard a `Result`. Every error must be handled or propagated using `?`.
* **Temporary Code:** No "TODO" logic or hardcoded hacks. Every commit must be state-of-the-art.

## ✅ The "Dos" (Good Practices)
* **Strong Typing:** Use the **Newtype pattern** (e.g., `struct PeerId(String)`) to prevent logic mix-ups.
* **Error Orchestration:** Use `thiserror` for modules and `anyhow` for high-level flow.
* **Asynchronous Mastery:** Use `tokio` for the P2P event loop (non-blocking I/O).
* **Structured Logging:** Log to `stdout` and a log file using `tracing` or `log` crates.
* **RAII:** Ensure DuckDB connections are pooled/scoped to prevent database locks.