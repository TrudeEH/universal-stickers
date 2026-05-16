# Universal Stickers

Small sticker/GIF picker built with GTK, libadwaita, and Rust.

![screenshot](image.png)

## Features

- Store imported images and GIFs in its own library
- Search stickers in a searchable responsive grid
- Rename or delete stickers
- Drag-and-drop support
- Copy selected sticker/GIF to the clipboard for use in other apps
- Export/import backups

## Project Layout

- `core/`: Rust storage library for import, search, rename, delete, thumbnail generation, and backups.
- `app/`: GTK4/libadwaita desktop app.
- `desktop/packaging/`: Linux desktop metadata, Flatpak manifest, and Arch/AUR recipe.
- `run.sh`: Linux helper script that builds and runs the app.
- `run.ps1`: Windows helper script that builds and runs the Rust app when GTK runtime dependencies are available.

## Dependencies

Install a current Rust toolchain with `cargo` and `rustup`.

The GTK desktop app needs:

- GTK 4 development files
- libadwaita development files
- pkg-config
- Optional: `xdg-desktop-portal-gnome` for the Ctrl+Meta+Space global shortcut on GNOME

The Rust core uses bundled SQLite through `rusqlite`, so SQLite development files are not required.

## Linux Setup

Ubuntu/Debian:

```bash
sudo apt-get install build-essential libgtk-4-dev libadwaita-1-dev pkg-config xdg-desktop-portal-gnome
```

Arch:

```bash
sudo pacman -S --needed rust gtk4 libadwaita pkgconf xdg-desktop-portal-gnome
```

Fedora:

```bash
sudo dnf install rust cargo gtk4-devel libadwaita-devel pkgconf-pkg-config xdg-desktop-portal-gnome
```

## Build And Run

From the repository root:

```bash
cargo run -p universal-stickers
```

Or use the helper script:

```bash
./run.sh
```

Build without launching:

```bash
./run.sh --no-run
```

The desktop executable is:

- Linux: `target/debug/universal-stickers`
- Windows: `target\debug\universal-stickers.exe`

## Tests

```bash
cargo test -p universal-stickers-core
cargo check -p universal-stickers
```

## Linux Install Staging

After a release build:

```bash
cargo build --release -p universal-stickers
DESTDIR="$PWD/pkgroot" PREFIX=/usr ./scripts/install-linux.sh
```

This stages the binary, desktop file, metainfo, icon, README, and license under
`pkgroot`.

## Arch / AUR Packaging

The Arch package recipe lives in
`desktop/packaging/arch/universal-stickers-git`.

From an Arch system:

```bash
cd desktop/packaging/arch/universal-stickers-git
makepkg -si
```

The release workflow also builds a `.pkg.tar.zst` artifact from this recipe.
For AUR publishing, upload `PKGBUILD` and a regenerated `.SRCINFO` to
`ssh://aur@aur.archlinux.org/universal-stickers-git.git`.
