<div align="center">
  <a href="https://getnodalia.com">
    <img src="https://raw.githubusercontent.com/danielmigueltejedor/nodalia-web/main/assets/brand/nodalia-wordmark-badge.svg" width="150" alt="Nodalia">
  </a>
  <br><br>
  <img src="https://raw.githubusercontent.com/danielmigueltejedor/letra/main/assets/app-icon/hicolor/512x512/apps/dev.danielmiguelt.Letra.png" width="192" alt="Letra">
  <h1>Letra</h1>
  <p><strong>The word processor of the Nodalia suite.</strong></p>

  <p>
    <img src="https://img.shields.io/badge/status-early%20stage-orange" alt="Early stage">
    <img src="https://img.shields.io/badge/Rust-native-dea584?logo=rust&logoColor=black" alt="Native Rust">
    <img src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-2ea44f" alt="MIT OR Apache-2.0">
  </p>

  <p>
    <a href="#overview">Overview</a> ·
    <a href="#build-and-run">Build</a> ·
    <a href="#project-status">Status</a> ·
    <a href="#the-nodalia-suite">Suite</a> ·
    <a href="#license">License</a>
  </p>
</div>

---

## Overview

**Letra** is the word processor of [Nodalia](https://getnodalia.com): a native app written in Rust,
with no account and no cloud. It runs on Linux, macOS and Windows.

> [!IMPORTANT]
> **Letra is at an early stage.** There are no published builds yet, and the Nodalia interface is
> not wired in.

## Build and run

You need a recent stable Rust toolchain.

```bash
git clone https://github.com/getnodalia/letra.git
cd letra
cargo run --release -p letra
```

## Project status

| Area | State |
|---|---|
| Package and binary names | Pending |
| App icon | Done: portrait of Miguel de Cervantes |
| Nodalia interface | Not started |
| Shared interface resources | Planned: the Nodalia apps will share interface components and styles |
| Installable packages | None published |

## App icon

Nodalia app icons are black-and-white portraits of historical figures over the app colour.
Letra's is Miguel de Cervantes, the foremost writer in Spanish. Provenance and licence are in [`assets/app-icon/`](assets/app-icon/).

## The Nodalia suite

Letra is one of the applications of [Nodalia](https://getnodalia.com), an independent software
project developed in public by [Daniel Miguel Tejedor](https://danielmigueltejedor.com).

## License

`MIT OR Apache-2.0`. See [`LICENSE-MIT`](LICENSE-MIT), [`LICENSE-APACHE`](LICENSE-APACHE) and
[`NOTICE`](NOTICE).
