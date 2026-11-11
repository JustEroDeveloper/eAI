# Security Policy

## Supported Versions

| Version | Supported |
| ------- | :-------: |
| eAI (macOS) | :white_check_mark: |
| eAI Lite (macOS) | :white_check_mark: |
| eAI Lite (Windows) | :white_check_mark: |
| eAI (Windows) | :x: (planned) |

## Reporting a Vulnerability

If you discover a security vulnerability in eAI or eAI Lite, please report it responsibly.

- **Discord:** `@realjustero` — [discord.gg/k9as2fGMES](https://discord.gg/k9as2fGMES)
- Please do not open public issues for security vulnerabilities until they have been addressed.

When reporting, include:
- A description of the issue and its potential impact.
- Steps to reproduce or proof-of-concept.
- The affected version/platform (eAI or eAI Lite, macOS/Windows).
- Any suggested fixes, if available.

You can expect an acknowledgement within a reasonable timeframe, and a fix will be released as soon as possible.

## Data Storage

- All user data is stored locally in `Documents/eAI/` (profiles, conversations, settings, avatars).
- No data is sent to external servers except search queries (SearXNG) and AI requests (Ollama), both of which run on your own machine.
- Passwords are hashed with SHA-256 and stored locally per profile.
- No telemetry or analytics are collected.

## Network & Privacy

- eAI communicates only with locally-running services: Ollama (port `11434`) and SearXNG (port `8080`).
- No third-party cloud APIs or tracking endpoints are used.
- LAN exposure of Ollama/SearXNG is opt-in via environment variables and is the user's responsibility to secure.

## Dependencies

- The project is built in Rust with `eframe/egui`, `tokio`, `reqwest`, and `serde`. Dependencies are managed via `Cargo.lock`.
- Please report any known vulnerable dependencies through the channels above.
