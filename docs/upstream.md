# Letra upstream

| Field | Value |
|---|---|
| Upstream | `storytold/wordcraft` |
| Last observed | `69ea44776d8e01fa96ad1c4766c2675896f4df01` |
| Last evaluated | `8548b33be1c9238e6d42fb2a72fe27ccdd56be7f` |
| Last integrated | `8548b33be1c9238e6d42fb2a72fe27ccdd56be7f` imported with full history on 2026-11-10 |
| Skipped | the community button and the brand files (Letra has neither) |
| Adapted | every name (`scripts/rebrand.py`), the app icon, the Nodalia interface |
| Conflicting | `README.md` (kept ours), `AGENTS.md` (upstream plus our quality contract) |

## How the sync works

Upstream's code arrives renamed: `scripts/rebrand.py` applies `.github/rebrand.toml` (WordCraft
becomes Letra, internal names included; links to the shared libraries point at
getnodalia/foundation) to upstream's tree, and the weekly sync
(`.github/workflows/upstream-sync.yml`) records the result on the `upstream-letra` branch. Each
commit there has the previous mirror commit and the upstream commit it renames as parents, and
names that commit in an `Upstream-Commit:` trailer. The sync merges the newest mirror commit
into a branch and opens a pull request; files listed in `.github/upstream-keep` keep this
repository's version.

The mirror starts at the renamed `8548b33b`, merged into `main` (with this repository's tree)
by the pull request that renamed the app. Before the first sync creates the `upstream-letra`
branch, the workflow finds that commit through its trailer. `python3 scripts/rebrand.py check`
fails when an old name comes back; a line that must keep one says `rebrand: keep`.
