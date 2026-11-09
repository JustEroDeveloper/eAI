# eAI and eAI Lite
**NOT FULLY STABLE STILL DEVELOPING**

A high-performance, native macOS and Windows desktop AI assistant built with **Rust**, **eframe/egui**, and **tokio**, featuring a sleek macOS-inspired dark mode interface and real-time local model streaming via **Ollama**.

## Pictures

### MacOS (eAI)
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


## Local Setup

To use **eAI** or **eAI Lite**, both **Ollama** and **SearXNG** must be installed and running on your host machine.

---

### Part 1: Installing & Setting Up Ollama

[Ollama](https://ollama.com/) handles local AI model inference on macOS and Windows.

1. **Download & Install:**
   - Download the installer from [ollama.com](https://ollama.com/) and follow the setup wizard.
2. **Pull a Model:**
   - Open your terminal or Command Prompt and download your preferred model:
     ```bash
     ollama pull qwen3:1.7b
     ```
3. *(Optional)* **Enable LAN Access:**
   - If eAI runs on the same PC as Ollama, no additional setup is required (`http://localhost:11434`).
   - To expose Ollama to other devices on your local network:
     - **Windows:** Quit Ollama via the system tray. Open **Environment Variables** in System Properties, create a new System/User Variable named `OLLAMA_HOST` with the value `0.0.0.0:11434`, then re-launch Ollama.
     - **macOS:** Open Terminal and execute `launchctl setenv OLLAMA_HOST "0.0.0.0:11434"`, then restart the Ollama desktop app.

---

### Part 2: Installing & Setting Up SearXNG (via Docker)

[SearXNG](https://github.com/searxng/searxng) provides privacy-respecting real-time web search capabilities for eAI.

> **Prerequisite:** Ensure **Docker Desktop** is installed and running on your system.

#### 1. Clone the Setup Repository
```bash
git clone [https://github.com/searxng/searxng-docker.git](https://github.com/searxng/searxng-docker.git)
cd searxng-docker
```
#### 2. Initialize Configuration Files
Run the container once to generate the default environment configs, then bring it down:
```bash
docker compose up -d
docker compose down
```
#### 3. Enable JSON API Output
Open searxng/settings.yml in a text editor, navigate to the search: block, and add - json under formats::
```yaml
search:
  safe_search: 0
  autocomplete: ""
  formats:
    - html
    - json
```
#### 4. Launch the Service
Start the SearXNG stack in background mode:
```bash
docker compose up -d
```
**Verify Installation:** Test the endpoint by visiting http://localhost:8080/search?q=test&format=json in your browser. You should receive a raw JSON response.

#### Part 3: Connecting eAI to Your Services
Configure eAI / eAI Lite settings using the appropriate endpoints:

| Service | Port | Same Machine Endpoint | Network (LAN) Endpoint |
| :--- | :--- | :--- | :--- |
| **Ollama** | `11434` | `http://localhost:11434` | `http://<HOST-IP-ADDRESS>:11434` |
| **SearXNG** | `8080` | `http://localhost:8080` | `http://<HOST-IP-ADDRESS>:8080` |
