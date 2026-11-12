# macOS DMG window

What Finder shows when the DMG opens: a 660 × 400 pt background with the app icon and the
`Applications` link side by side. `package.sh` copies these files into the image; nothing here is
generated at build time, so the DMG builds with a plain `hdiutil create -srcfolder` (no Finder
scripting on CI, nothing extra installed in the signing job).

Nothing in this step may touch `Letra.app`: `codesign --verify --deep --strict` rejects any
`com.apple.FinderInfo` or resource fork on the bundle. `hdiutil makehybrid -hfs`, which built the
image before, put a FinderInfo on every file in it, so the released app failed that check (#525).
`package.sh` clears extended attributes before signing and, after building the DMG, mounts it
read-only and checks the app inside it, and a `ditto` copy of it, with `--strict`.

| File | What |
|---|---|
| `background.svg` | Source of the background, drawn by `tools/dmg-background/draw_background.py` in [nodalia-design](https://github.com/getnodalia/nodalia-design): two Nodalia islands on the window colour, the left one in the Letra colour (`#3A9EE6`) with a flat picture of what Letra does, the right one white under the icons. Its text (Sora) is outlined, so rendering it needs no fonts. |
| `background.tiff` | The background at 1x (660 × 400 px, 72 dpi) and 2x (1320 × 800 px, 144 dpi) in one HiDPI TIFF (16-colour palette, Deflate, sRGB). Goes to `.background/background.tiff`. |
| `dmg-layout.DS_Store` | Finder's view settings for the volume: window size, icon size 128, Letra.app at (326, 205), `Applications` at (574, 205), and the background. Goes to `.DS_Store` in the image; named so it isn't mistaken for (or ignored like) a Finder-generated `.DS_Store`. |
| `generate.py` | Writes `background.tiff` and `dmg-layout.DS_Store` from the SVG and the layout above. |

## The volume name has no version

The mounted volume is called `Letra`, not `Letra <version>`. The layout points at the background
through an alias that includes the volume name, and Finder resolves it by that name: with a
versioned name the window keeps its size and icon positions but shows no background (tested).
The DMG file name (`wordcraft-<version>-macos-<arch>.dmg`) still carries the version.

## Rules

- **Finder draws the icon labels in black in light and dark mode** when a window has a background,
  so the area under both icons stays light (the white island).
- **Nothing goes inside the icon boxes:** artwork keeps 10 pt clear of each 128 pt icon box and of
  the label strip under it.

## Regenerate

Regenerate `background.svg` with nodalia-design's `tools/dmg-background/draw_background.py` (or the layout constants in `generate.py`), then run, on any OS:

```sh
pip install 'pillow>=12' ds_store==1.3.3 mac_alias==2.2.3
python3 packaging/macos/dmg/generate.py   # needs resvg on PATH
```

Both outputs are byte-for-byte reproducible and carry nothing from the machine that ran it: the
SVG renders without any fonts, and `dmg-layout.DS_Store` is written from scratch, its background
alias holding only the volume name and `/.background/background.tiff`. If you add text to the SVG,
outline it (`usvg` from resvg converts text to paths). The window is 660 × 432 with Finder's 32 pt
title bar; the content area is 660 × 400.

This follows Pluma's `packaging/macos/dmg/` (storytold/vectorcraft#493).
