# Product Requirements Document (PRD): SocialKube
**Project:** SocialKube (P2P Compute Collective)
**Version:** 1.0 (Rust Implementation)
**Status:** Approved for Development

---

## 1. Executive Summary
SocialKube is a decentralized desktop application that enables peer-to-peer LLM (Large Language Model) inference. Users contribute local GPU/CPU resources to a shared "Kube" and receive access to high-parameter models in return. The system uses a credit-based economy to ensure fair distribution of compute.

## 2. Technical Goals (The "Kube" Core)
### 2.1 Distributed Inference
* **Dynamic Sharding:** The system must split models (e.g., Llama-3-70B) into computational blocks. No single node is required to hold the full model.
* **Heterogeneous Support:** Must utilize NVIDIA (CUDA), Apple Silicon (Metal), and CPU (AVX/AMX) via the `llama.cpp` or `candle` backend.
* **Fault Tolerance:** If a peer disconnects during a "forward pass," the request must automatically reroute to an alternative peer holding the same shard.

### 2.2 Networking & Discovery
* **Protocol:** Built on `libp2p`.
* **NAT Traversal:** Must use Kademlia DHT and Relay nodes to connect users across different Israeli ISPs (Bezeq, Hot, Cellcom) without manual port forwarding.
* **Peer Scoring:** A local "reputation" system to prioritize low-latency peers for complex coding tasks.

## 3. Functional Requirements

### 3.1 The SocialKube Agent (Rust Service)
| ID | Requirement | Description |
| :--- | :--- | :--- |
| **FR-1** | **Participation Toggle** | A manual switch in the UI/System Tray to enable/disable worker mode. |
| **FR-2** | **Resource Guard** | The agent must allow users to cap VRAM/RAM usage (e.g., "Use only 4GB"). |
| **FR-3** | **Hardware Benchmarking** | Auto-detect TFLOPS and memory bandwidth to determine optimal shard assignment. |

### 3.2 The SocialKube Hub (Web UI)
| ID | Requirement | Description |
| :--- | :--- | :--- |
| **FR-4** | **Credit Wallet** | Real-time display of earned vs. spent credits. |
| **FR-5** | **Model Selector** | Dropdown showing models available in the current swarm and their "Credit Cost." |
| **FR-6** | **Chat Interface** | A clean, markdown-enabled chat window with code syntax highlighting. |

### 3.3 Economy & Incentives
* **Contribution Credits:** 1 Token processed for the network = $X$ Credits earned.
* **Inference Cost:** 1 Token requested from the network = $Y$ Credits spent (where $Y$ scales with model size).
* **Multi-Model Hosting:** The software autonomously decides which shards to host based on global demand to ensure "Swarm Health."

## 4. Privacy & Security
* **Encrypted Tunnels:** All prompt data is encrypted between the client and the specific workers.
* **Shard Privacy:** Workers process numeric tensors; they do not reconstruct the full human-readable prompt unless they host the "Embedding" (first) and "De-embedding" (last) layers.
* **Integrity Checks:** Periodic "Canary Tasks" to verify that workers aren't returning hallucinated/fake data to farm credits.

## 5. User Journey
1. **The Setup:** User installs the Rust binary. The agent performs a one-time hardware scan.
2. **The Socializing:** User toggles "Participate." The node connects to the Israeli P2P swarm and caches 4-8GB of shards.
3. **The Payoff:** After "seeding" for an hour, the user has earned enough credits to use the Llama-3-405B model for a complex coding architecture task.
4. **The Interaction:** User chats via `localhost:3000`. The prompt hops across 5-10 peers; the answer streams back token-by-token.

## 6. Success Metrics
* **Discovery Speed:** Finding at least 5 peers within 10 seconds of startup.
* **Inference Latency:** Total overhead of P2P routing should not exceed 500ms per token over WAN.
* **Stability:** 0% data corruption during cross-city shard transfers.