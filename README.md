# eAI and eAI Lite
**NOT OUT YET STILL DEVELOPING**

A high-performance, native macOS and Windows desktop AI assistant built with **Rust**, **eframe/egui**, and **tokio**, featuring a sleek macOS-inspired dark mode interface and real-time local model streaming via **Ollama**.

## Pictures

### MacOS (eAI)
<img width="1444" height="696" alt="aaadagda" src="https://github.com/user-attachments/assets/94c5645f-6860-4bf7-8bc9-16407e47e2b3" />
<img width="1806" height="1228" alt="image" src="https://github.com/user-attachments/assets/893cde54-0a94-40c0-9c2c-05d9daacde71" />

### Windows (eAI Lite)
<img width="2078" height="1432" alt="image" src="https://github.com/user-attachments/assets/4800f503-9d16-4df6-b771-c21f49562f79" />

---

## Features

* **Native macOS Aesthetics:** Custom dark-mode theme designed to mirror modern macOS design guidelines, including split sidebar chat history, capsule input bars, and polished message bubbles.
* **Local LLM Integration:** Connects directly to a local [Ollama](https://ollama.com/) instance for privacy-focused, lightning-fast inference.
* **Real-time Streaming:** Asynchronous message streaming via Tokio channels and `reqwest` for smooth, typewriter-style response generation.
* **Session Management:** Create and switch between multiple chat conversations instantly.
* **Interactive UI Elements:** Model selector dropdown, quick prompt suggestion chips, and dynamic thinking indicators.

---

## Tech Stack

* **UI Framework:** [`eframe`](https://github.com/emilk/egui) / [`egui`](https://github.com/emilk/egui)
* **Async Runtime:** [`tokio`](https://tokio.rs/)
* **HTTP Client & Serialization:** [`reqwest`](https://github.com/seanmonstar/reqwest), [`serde`](https://serde.rs/), [`serde_json`](https://github.com/serde-rs/json)
* **Utilities:** [`uuid`](https://github.com/uuid-rs/uuid), [`futures-util`](https://github.com/rust-lang/futures-rs)

---

## Prerequisites

Before running the application, make sure you have:
1. **Rust & Cargo** installed if you want to use eAI Lite ([Install Rust](https://www.rust-lang.org/tools/install))
2. **SearXNG** running locally on your computer with Docker Desktop or IDE.
3. **Ollama** running locally on your machine with at least one model downloaded (e.g., `llama3.2` or `qwen3`):

   ```bash
   ollama serve
