# Nodalia migration: Letra

Category: secondary app.
Status: renamed to Letra throughout, own release workflows, as of 2026-11-13. Evidence: the pull
request that adds this file.

## Current state

Pure-Rust word processor (egui), MIT OR Apache-2.0. Cargo git dependencies come from
getnodalia/foundation and nodalia-design. The gate, FreeBSD, Nix, packaging lint, AGENTS.md and
git-sources checks call getnodalia/foundation's reusable workflows pinned by commit. Releases run
on Letra's own workflows (`release-auto.yml` with Foundation's release-tag and release-publish,
then `release-linux.yml`, `release-macos.yml` and `release-windows.yml`). Every name is Letra's,
internal ones included: `scripts/rebrand.py` with `.github/rebrand.toml` renames upstream's tree,
the weekly sync merges it renamed (`docs/upstream.md`), and CI fails if an old name comes back.

Letra keeps `.docx` as its document format (decision of 2026-11-13): upstream never had a file
format of its own, so there is no old extension to rename.

## Inherited dependencies still pending

| Dependency | Where | Replacement | Blocked on |
|---|---|---|---|
| Spelling language packs downloaded from `storytold/craft-i18n` releases | `crates/engine/src/packs.rs`, `crates/engine/src/packs/store.rs` | the same packs published from getnodalia/foundation | a Foundation workflow that publishes the packs as release files |
| upstream's repository (`docs/upstream.md`) | `.github/workflows/upstream-sync.yml` | none: the weekly sync reads upstream and renames it | — |

Resolved: craft-common-actions (release, contributors refresh, Windows ARM64) is gone; fonts,
translations and the shared libraries come from getnodalia/foundation's vendor branches; the
`[patch]` for `storytold/winprint.rs` in `Cargo.toml` only names the old home of crates fetched
from getnodalia/foundation.

## Technical debt

- Windows ARM64 is not packaged yet (the inherited workflow was removed); `release-windows.yml`
  builds x64.
- The translation catalog in craft-i18n is still named `wordcraft.tsv` (`.github/rebrand.toml`
  keeps that name).

## Reusable Nodalia components

getnodalia/foundation (vendored crates, fonts, translations, reusable workflows);
getnodalia/nodalia-design (theme, icons, `nodalia-update`).

## NDL work pending

The interface in the Nodalia Design Language: islands, the hamburger menu with About,
Preferences and Keyboard shortcuts, NDL window controls on Linux only, the global menu over D-Bus.

## Recommended order

1. The NDL interface.
2. Language packs published from Foundation.
3. Windows ARM64 packages in `release-windows.yml`.
