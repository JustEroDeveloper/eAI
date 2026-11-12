## [Beta 7.4.2] - 2026-11-12

### Fixed
- macOS installer window no longer opens fullscreen; now opens at 1100x730.

## [Beta 7.4.1] - 2026-11-11

### Added
- Redesigned macOS installer: the `.dmg` now opens with a drag-to-Applications window.
### Changed
- Installer window is smaller (1100x730) instead of filling the screen.

## [Beta 7.4] - 2026-11-11

### Added
- Language support: English (default), Türkçe, Deutsch, Español, Français. Pick from the login screen or Settings → General; saved globally.
- `.jar` / `.zip` attachments: entry listing plus text/code file contents sent to the model, with a "Save a copy" button to export the original archive.

## [Beta 7.3] - 2026-11-11

### Added
- Delete account: permanently remove a profile from Settings → Account (two-step confirm), deleting its chats, settings and avatar files.

### Changed
- Web search toggle moved to a minimal icon in the top-right above the message bar; accent dot shows when it's on.

## [Beta 7.2] - 2026-11-11

### Fixed
- Upgraded the `zip` crate from 2.6.1 to 9.5.1, fixing three Dependabot advisories (zip-cloaking RCE, unbounded symlink-target handling, AES entry HMAC bypass).
- Removed the last hard-coded cryptographic value in `account.rs` tests.

## [Beta 7.1] - 2026-11-11

### Fixed
- Password hashing upgraded from iterated SHA-256 to PBKDF2-HMAC-SHA256 (100,000 iterations) with a per-profile random salt.
- Removed hard-coded cryptographic values from tests (dynamic, randomly generated test data instead).

## [Beta 7] - 2026-11-11

### Added
- Drag & drop files into the message bar to attach them (images and text/code files).
- Clear warning shown in the composer when a file can't be attached, explaining why.

### Changed
- Attachment flow now matches the `+` picker (768px image resize, 8 MB limit).

## [Beta 6] - 2026-11-11

### Added
- Full source code published to the `main` branch.
- Refreshed `SECURITY.md` with improved formatting and expanded context.
- macOS `.app` bundle with icon for eAI Lite.

### Changed
- Rebuilt macOS and Windows binaries from source.

### Fixed
- Eliminated dead-code warnings in `archive.rs` and `net.rs`.

PR: https://github.com/JustEroDeveloper/eAI/pull/6

# Changelog

All notable changes to **eAI Lite** are documented here.

## [Beta 5] - 2026-11-10

### Added
- Attach `.txt` and code files through the `+` icon in the message bar.
- Removable "file attached" chips in the composer and message bubbles.
- Censorless mode toggle in **Settings → General**.
- External API provider support in **Settings → API** (Ollama or OpenAI-compatible).
- macOS app bundle (`.dmg` / `.zip`) alongside Windows `.exe`.
- Broader Unicode font fallbacks to reduce empty-box/tofu glyphs.

### Improved
- UI overhaul: refreshed dark palette, larger rounding, cleaner cards, input bar, and message bubbles.
- Thought process header now uses plain ASCII arrows instead of missing glyphs.
- Send button enables when a file is attached even without text.

### Fixed
- Empty-box rendering for unsupported Unicode characters in generated text and thinking output.
- File contents are now included in the prompt sent to the model.

## [Beta 4] - 2026-11-10

### Added
- Local profiles and sign-in flow.
- Per-profile settings and conversations.

## [Beta 3] - 2026-11-10

### Added
- Web search integration and source citations.
- Thinking/reasoning view for supported models.

## [Beta 2] - 2026-11-09

### Added
- Model renaming and settings panels.
- Image attachments.

## [Beta 1] - 2026-11-09

### Added
- Initial Windows release of eAI Lite.
