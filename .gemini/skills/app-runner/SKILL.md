---
name: app-runner
description: DevOps and general engineer. Triggered when requested to run the app, simulate the P2P network, or manage Docker containers.
---
# Knowledge & Instructions
(Insert the App Runner & General Engineer "Dos" and "Don'ts" here...)
# Skill: App-Runner & General Engineer
**Context:** Invoked for orchestration, simulation, and general code integrity.
**Mandate:** Automate the P2P ecosystem and maintain overall software quality.

## 🚫 The "Don'ts" (Bad Practices)
* **WET Code (Write Everything Twice):** Abstract repeated logic between `p2p` and `engine` into shared utilities.
* **Forced Testing:** Do not write tests for standard library behavior or simple getters. Only test business logic.
* **Magic Numbers:** Never use raw integers for ports or timeouts. Use a `Config` struct or `.env`.
* **Deep Nesting:** Avoid "Arrow Code." Use guard clauses and early returns to keep logic readable.

## ✅ The "Dos" (Good Practices)
* **Multi-Peer Simulation:** Use Docker for multiple instances with dynamic port mapping:
    * Peer A: API `8080`, P2P `4001`, Volume `db_1`
    * Peer B: API `8081`, P2P `4002`, Volume `db_2`
* **Business Logic Testing:** Write tests *only* for: Social Score math, credit validation, and P2P parsing.
* **Short Docs:** Every function must have a 1-2 line documentation block explaining its purpose.
* **Production-Ready Docker:** Use multi-stage builds (build in Rust, run in `debian-slim`).
* **Standardized Output:** Ensure all services output logs in a consistent, cross-referenceable format.