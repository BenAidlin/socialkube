# Project: SocialKube (P2P AI Swarm)

## Project Overview
SocialKube is a decentralized P2P compute network written in Rust. It allows users to contribute GPU/CPU power to a swarm in exchange for credits to use large LLMs.

## Core Technologies
- **Language:** Rust (Stable)
- **Networking:** libp2p (Kademlia DHT, Gossipsub, mDNS)
- **AI Engine:** Candle / llama.cpp (Targeting GGUF/Safetensors)
- **Frontend:** Next.js (Localhost:3000)
- **Database:** DuckDB (Embedded for high-performance analytics and credits)

## Coding Standards
- **Logging:** Do NOT use `println!`. Use the `log` or `tracing` crates. Logs must output to both stdout and a rolling log file.
- **Style:** Idiomatic, production-ready Rust. Prioritize safety and 'Send/Sync' traits.
- **Functions:** Keep functions short and concise. Every function must have a brief doc comment (`///`) explaining its purpose.
- **Testing:** Create tests for core business logic. Do not "force" tests for trivial code; focus on necessary coverage.
- **Async:** Use 'tokio' for the runtime.
- **Maintenance:** After significant design changes, this `GEMINI.md` file must be updated to stay in sync with the architecture.

## Instructions for Gemini CLI
1. **Context Awareness:** Always check @src/p2p/behaviour.rs before suggesting networking changes.
2. **Step-by-Step:** For complex features (like sharding logic), provide a checklist first.
3. **P2P Priority:** When designing features, consider the "Offline" state (what happens if a peer leaves?).

## Key Files
- @src/main.rs: Main entry and swarm loop.
- @src/p2p/behaviour.rs: Custom network behaviour logic.
- @src/p2p/host.rs: Node identity and swarm initialization.
- @src/economy/ledger.rs: DuckDB implementation for credits.