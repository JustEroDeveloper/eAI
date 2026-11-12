# eAI & eAI Lite

[![Release](https://img.shields.io/github/v/release/JustEroDeveloper/eAI?include_prereleases&label=release&color=blue)](https://github.com/JustEroDeveloper/eAI/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](./LICENSE)
[![Platform](https://img.shields.io/badge/Platform-macOS%20%7C%20Windows-lightgrey)](#-installation)


# REPO WAS MOVED TO : https://github.com/embeddiumAI
> ⚠️ **NOT FULLY STABLE — STILL IN DEVELOPMENT**

A high-performance, privacy-focused, native macOS and Windows local AI client built with **Rust**, **eframe/egui**, **Swift** and **tokio** — featuring a sleek macOS-inspired dark-mode interface and real-time local model streaming via **Ollama**.

**Discord:** [discord.gg/k9as2fGMES](https://discord.gg/k9as2fGMES) — **@realjustero**

You can use the source code to improve or modify this application — I don't mind. This project is a hobby, not for any earnings whatsoever. Contact me on Discord if you find bugs or errors!

---

## 📸 Pictures

### eAI (macOS)

<img width="1806" height="1228" alt="image" src="https://github.com/user-attachments/assets/893cde54-0a94-40c0-9c2c-05d9daacde71" />

> ℹ️ Regular eAI for Windows is coming at a later date — I'm currently fixing bugs with eAI Lite.

### eAI Lite (Windows & macOS)

<img width="2074" height="1444" alt="image" src="https://github.com/user-attachments/assets/9fcdcd9a-dd48-49e0-82e5-9e224a879943" />

---

## ✨ Features

- **Local Profiles** — first-run profile picker with an optional password keeps each person's chats, avatar and settings separate on the same machine.
- **Native macOS Aesthetics** — custom dark-mode theme mirroring modern macOS design: split sidebar chat history, capsule input bars, and polished message bubbles.
- **Local LLM Integration** — connects directly to a local [Ollama](https://ollama.com/) instance for privacy-focused, lightning-fast inference.
- **Real-time Streaming** — asynchronous message streaming via Tokio channels and `reqwest` for smooth, typewriter-style responses.
- **Session Management** — create and switch between multiple chat conversations instantly.
- **Interactive UI** — model selector dropdown, quick prompt suggestion chips, and dynamic thinking indicators.
- **Drag & Drop Attachments** — drop images or text/code files straight into the message bar (with clear warnings when a file can't be attached).

---

## 🛠 Tech Stack

| Component | Library |
| :--- | :--- |
| UI Framework | [`eframe`](https://github.com/emilk/egui) / [`egui`](https://github.com/emilk/egui) |
| Async Runtime | [`tokio`](https://tokio.rs/) |
| HTTP & Serialization | [`reqwest`](https://github.com/seanmonstar/reqwest), [`serde`](https://serde.rs/), [`serde_json`](https://github.com/serde-rs/json) |
| Utilities | [`uuid`](https://github.com/uuid-rs/uuid), [`futures-util`](https://github.com/rust-lang/futures-rs) |

---

## 📦 Installation & Setup

To use **eAI** or **eAI Lite**, both **Ollama** and **SearXNG** must be installed and running on your host machine.

**Prerequisites:**
1. **Rust & Cargo** (for building eAI Lite from source) — [Install Rust](https://www.rust-lang.org/tools/install)
2. **SearXNG** running locally via Docker Desktop.
3. **Ollama** running locally with at least one model pulled (e.g. `llama3.2` or `qwen3`).

### Part 1: Ollama

[Ollama](https://ollama.com/) handles local AI model inference on macOS and Windows.

1. **Download & Install** — grab the installer from [ollama.com](https://ollama.com/) and follow the setup wizard.
2. **Pull a model:**
   ```bash
   ollama pull qwen3:1.7b
   ```
3. *(Optional)* **Enable LAN access** — if eAI runs on the same PC as Ollama, no setup is needed (`http://localhost:11434`). To expose it to your network:
   - **Windows:** Quit Ollama from the system tray → **Environment Variables** in System Properties → create a variable `OLLAMA_HOST` = `0.0.0.0:11434` → relaunch Ollama.
   - **macOS:** `launchctl setenv OLLAMA_HOST "0.0.0.0:11434"`, then restart the Ollama desktop app.

### Part 2: SearXNG (via Docker)

[SearXNG](https://github.com/searxng/searxng) provides privacy-respecting real-time web search.

> **Prerequisite:** [Docker Desktop](https://www.docker.com/products/docker-desktop/) installed and running.

1. **Clone the setup repository:**
   ```bash
   git clone https://github.com/searxng/searxng-docker.git
   cd searxng-docker
   ```
2. **Initialize config files:**
   ```bash
   docker compose up -d
   docker compose down
   ```
3. **Enable the JSON API** — open `searxng/settings.yml`, find the `search:` block, and add `- json` under `formats:`:
   ```yaml
   search:
     safe_search: 0
     autocomplete: ""
     formats:
       - html
       - json
   ```
4. **Launch:**
   ```bash
   docker compose up -d
   ```

**Verify:** visit `http://localhost:8080/search?q=test&format=json` — you should get a raw JSON response.

### Part 3: Connect eAI to Your Services

| Service | Port | Same Machine | Network (LAN) |
| :--- | :--- | :--- | :--- |
| **Ollama** | `11434` | `http://localhost:11434` | `http://<HOST-IP-ADDRESS>:11434` |
| **SearXNG** | `8080` | `http://localhost:8080` | `http://<HOST-IP-ADDRESS>:8080` |

### Building from Source

```bash
git clone https://github.com/JustEroDeveloper/eAI.git
cd eAI/App
cargo build --release
./target/release/eai-lite
```

---

## 🔒 Privacy

- No accounts, no analytics, no telemetry, no tracking — all data stays local in `Documents/eAI/`.
- See [`DISCLAIMER.md`](./DISCLAIMER.md) for the full privacy disclaimer and [`SECURITY.md`](./SECURITY.md) for the security policy.

## 📄 License

eAI Lite is open source under the [MIT License](./LICENSE). See [`DISCLAIMER.md`](./DISCLAIMER.md) for warranty and liability terms.
