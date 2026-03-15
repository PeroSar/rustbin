<div align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/logo.png">
    <source media="(prefers-color-scheme: light)" srcset="assets/logo-black.png">
    <img src="assets/logo-black.png" alt="rustbin logo" width="96">
  </picture>
  <h1>rustbin</h1>
  <p>A minimalist pastebin and URL shortener written in Rust.</p>

  [![CI](https://github.com/perosar/rustbin/actions/workflows/rust.yml/badge.svg)](https://github.com/perosar/rustbin/actions/workflows/rust.yml)
  [![Rust](https://img.shields.io/badge/rust-2024%20edition-orange)](https://www.rust-lang.org)
  [![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
</div>

---

## Features

- **Syntax highlighting** — 400+ languages powered by [syntect](https://github.com/trishume/syntect) with a GitHub Dark theme
- **Language detection** — automatic detection via filename extension or [Enry](https://github.com/go-enry/enry) classifier (Go FFI)
- **Markdown rendering** — GitHub Flavoured Markdown compatible with syntax-highlighted code blocks
- **URL shortening** — paste a URL to get a short redirect link instantly
- **Social previews** — generates 1200×630 PNG preview images for social media embeds
- **Paste expiration** — set optional TTLs; expired pastes are cleaned up automatically
- **LRU caching** — rendered HTML and preview images are cached in memory
- **Line linking** — click a line number to link to it; shift-click to select a range
- **Single binary** — assets, fonts, and syntax definitions are all embedded

## Installation

### From source

**Prerequisites:** Rust (edition 2024) and Go (for the Enry FFI).

```bash
git clone https://github.com/perosar/rustbin
cd rustbin
cargo build --release
```

The compiled binary will be at `target/release/rustbin`.

## Usage

### Running the server

```bash
cp .env.example .env
# Edit .env as needed
./target/release/rustbin
```

The server starts on `http://0.0.0.0:3000` by default.

### Creating pastes

**Web UI** — open the root URL in your browser and paste content into the textarea.

**curl:**
```bash
# Create a paste from a file
curl -F 'file=@main.rs' https://example.com

# Create a paste from stdin
echo 'fn main() {}' | curl -F 'file=@-' https://example.com

# Create a paste with an expiration
curl -F 'file=@script.py' -F 'expires_in=24h' https://example.com

# Shorten a URL
curl -F 'file=@-' https://example.com <<< 'https://example.com/very/long/url'
```

The response contains the paste URL, e.g. `https://example.com/aBcDeFgHiJ`.

### Accessing pastes

| Endpoint | Description |
|---|---|
| `GET /{id}` | Rendered paste with syntax highlighting |
| `GET /{id}.{ext}` | Rendered paste with a language hint (e.g. `/{id}.go`) |
| `GET /{id}/raw` | Raw plain-text content |
| `GET /{id}/preview.png` | 1200×630 PNG preview image |

## Configuration

All configuration is done through environment variables (or a `.env` file).

| Variable | Default | Description |
|---|---|---|
| `DATABASE_URL` | `sqlite://rustbin.db` | SQLite connection string |
| `HOST` | `0.0.0.0` | Bind address |
| `PORT` | `3000` | Bind port |
| `MAX_PASTE_SIZE` | `2MB` | Maximum upload size (supports `KB`, `MB`, `GB`) |
| `RENDER_CACHE_CAPACITY` | `128` | Number of rendered HTML entries to cache |
| `CLEANUP_INTERVAL` | `3600` | Seconds between expired-paste cleanup runs |
| `DB_MIN_CONNECTIONS` | `1` | SQLite connection pool minimum |
| `DB_MAX_CONNECTIONS` | `5` | SQLite connection pool maximum |
| `RUST_LOG` | `rustbin=info` | Log level filter (uses `tracing-subscriber`) |

