<div align="center">
  <a href="https://getnodalia.com">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/danielmigueltejedor/nodalia-web/main/assets/brand/nodalia-wordmark-white.svg">
      <img src="https://raw.githubusercontent.com/danielmigueltejedor/nodalia-web/main/assets/brand/nodalia-wordmark-black.svg" width="150" alt="Nodalia">
    </picture>
  </a>
  <br><br>
  <img src="https://raw.githubusercontent.com/getnodalia/letra/main/assets/app-icon/hicolor/512x512/apps/dev.danielmiguelt.Letra.png" width="192" alt="Letra">
  <h1>Letra</h1>
  <p><strong>The word processor of the Nodalia suite.</strong></p>

  <p>
    <img src="https://img.shields.io/badge/status-early%20stage-orange" alt="Early stage">
    <img src="https://img.shields.io/badge/Rust-native-dea584?logo=rust&logoColor=black" alt="Native Rust">
    <img src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-2ea44f" alt="MIT OR Apache-2.0">
  </p>

  <p>
    <a href="#overview">Overview</a> ·
    <a href="#download">Download</a> ·
    <a href="#build-and-run">Build</a> ·
    <a href="#project-status">Status</a> ·
    <a href="#the-nodalia-suite">Suite</a> ·
    <a href="#license">License</a>
  </p>
</div>

---

<p align="center">
  <img src="https://raw.githubusercontent.com/getnodalia/nodalia-web/main/assets/screens/letra/letra-home-dark.jpg" width="900" alt="A document open in Letra, in the dark theme">
</p>

## Overview

**Letra** is the word processor of [Nodalia](https://getnodalia.com): a native app written in Rust,
with no account and no cloud, for letters, reports, essays and books. It runs on Linux, macOS and
Windows, and works with `.docx` documents.

- **Writing and formatting:** fonts, paragraph and character styles, lists, columns, tab stops and
  a ribbon with everything at hand.
- **Real pages:** headers and footers, page numbers, sections, footnotes, tables of contents and
  page breaks, in a faithful print layout.
- **Tables, images and equations,** in line or beside the text.
- **Review:** spelling and grammar in several languages, comments, track changes and document
  comparison.
- **Files:** `.docx` first, plus ODT, RTF, HTML, Markdown and plain text; exports to PDF.
- **Nodalia Design:** rounded islands, Letra's blue, NDL icons, round window controls on Linux
  (native ones on macOS and Windows), a light or dark theme chosen in Preferences, and its menus
  in the desktop's top bar on Linux desktops that have one.
- **Everything is a command:** every action can be scripted with `letra-cli` or driven by AI
  agents through MCP.

> [!IMPORTANT]
> **Letra is at an early stage.** Keep a copy of documents that matter.

<p align="center">
  <img src="https://raw.githubusercontent.com/getnodalia/nodalia-web/main/assets/screens/letra/letra-home-light.jpg" width="440" alt="The same document in the light theme">
  <img src="https://raw.githubusercontent.com/getnodalia/nodalia-web/main/assets/screens/letra/letra-review-dark.jpg" width="440" alt="The Review tab: spelling, comments and track changes">
</p>

## Download

Packages for every platform are on the [releases page](https://github.com/getnodalia/letra/releases/latest)
and on [getnodalia.com/letra](https://getnodalia.com/letra/):

| System | File |
|---|---|
| Linux x86_64 | AppImage, `.deb`, `.rpm`, `.tar.gz` |
| macOS (Apple Silicon and Intel) | Universal DMG |
| Windows x64 | MSI installer, portable zip |

The macOS app is not notarized: the first time, right-click it and choose **Open**. Letra checks
for new versions itself and offers to install them.

## Build and run

You need a recent stable Rust toolchain.

```bash
git clone https://github.com/getnodalia/letra.git
cd letra
cargo run --release -p letra
```

`cargo run --release -p letra -- --sample` opens a sample document. Headless use, the control
channel and MCP are described in [`docs/mcp.md`](docs/mcp.md) and
[`docs/control-protocol.md`](docs/control-protocol.md).

## Project status

Where Letra stands, feature by feature, is in [`ROADMAP.md`](ROADMAP.md) and
[`docs/gaps.md`](docs/gaps.md).

## App icon

Nodalia app icons are black-and-white portraits of historical figures over the app colour.
Letra's is Miguel de Cervantes, the foremost writer in Spanish. Provenance and licence are in [`assets/app-icon/`](assets/app-icon/).

## The Nodalia suite

Letra is one of the applications of [Nodalia](https://getnodalia.com), an independent software
project developed in public by [Daniel Miguel Tejedor](https://danielmigueltejedor.com).

## License

`MIT OR Apache-2.0`. See [`LICENSE-MIT`](LICENSE-MIT), [`LICENSE-APACHE`](LICENSE-APACHE) and
[`NOTICE`](NOTICE).
