---
name: nextjs-developer
description: Senior Next.js frontend developer. Triggered when building UI components, managing App Router state, or integrating with the Rust API.
---
# Knowledge & Instructions
(Insert the Next.js Developer "Dos" and "Don'ts" here...)
# Skill: Next.js Developer
**Context:** Invoked for all UI and frontend architecture tasks.
**Mandate:** Build a lean, responsive UI for monitoring the decentralized network.

## 🚫 The "Don't"s (Bad Practices)
* **Client Component Overuse:** Don't put `'use client'` at the top of every file. Keep data fetching on the server.
* **Prop Drilling:** Do not pass "Social Score" through 5+ nested components. Use **Zustand** or **Context** for global state.
* **Hardcoded API Endpoints:** Never hardcode `localhost:8080`. Use environment variables for multi-peer support.
* **Unoptimized Assets:** Avoid standard `<img>` tags; use `next/image` for lazy loading and optimization.
* **Manual State Sync:** Don't use `setInterval` for polling. Use **SWR** or **React Query** for revalidation.

## ✅ The "Dos" (Good Practices)
* **App Router Architecture:** Leverage `layout.tsx` for persistent sidebars and `page.tsx` for views.
* **Suspense Boundaries:** Use `<Suspense>` with skeleton loaders for P2P network responses.
* **Type-Safe Props:** Define TypeScript interfaces for every component and API response—no `any`.
* **Atomic Design:** Break UI into small, reusable components (e.g., `ScoreBadge`, `PeerCard`).
* **Tailwind/Shadcn:** Use utility-first CSS to keep styles scoped and consistent.