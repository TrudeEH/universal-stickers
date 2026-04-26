# Universal Stickers

Small sticker/GIF picker built with QT and Rust.

## Features

- Store imported images and GIFs in its own library
- Search stickers in a searchable grid
- Rename or delete stickers
- Drag-and-drop import support
- Copy selected sticker to clipboard for use in other apps
- Export/import backups

## Project Layout

- `core/`: Rust storage library for import, search, rename, delete, thumbnail generation, and backups.
- `ffi/`: `cxx` bridge crate that exposes the Rust core to the desktop app.
- `desktop/`: Qt Widgets desktop app.
- `run.ps1`: Windows helper script that configures, builds, deploys Qt runtime files, and runs the app.

## Dependencies

## Rust

Install a current Rust toolchain with `cargo` and `rustup`.

This repo uses:

- `cargo`
- `rustup`

On Windows with the MinGW Qt build used by this repo, also install the GNU Rust target:

```powershell
rustup target add x86_64-pc-windows-gnu
```

## Desktop Build Dependencies

The desktop app needs:

- CMake 3.21+
- Ninja
- Qt 6 with at least:
  - `Core`
  - `Gui`
  - `Widgets`
- Optional:
  - `DBus`
  - `KF6GlobalAccel` for KDE global hotkey support on Linux

The Rust core uses bundled SQLite through `rusqlite`, so you do not need to install SQLite separately.

## Windows Setup

For the helper script path, install:

- Rust with `cargo` and `rustup`
- Python 3 with `pip`

On first run, `run.ps1` bootstraps a repo-local Qt toolchain under `.tools/qt` and installs:

- CMake under `.tools/qt/Tools/CMake_64`
- MinGW under `.tools/qt/Tools/mingw1310_64`
- Qt 6.8.3 under `.tools/qt/6.8.3/mingw_64`

If the GNU Rust target is missing, the script also runs:

```powershell
rustup target add x86_64-pc-windows-gnu
```

If you already have your own Qt installation elsewhere, you can still build manually by pointing CMake at your Qt prefix instead of using `run.ps1`.

## Linux Setup

Install:

- Rust toolchain
- CMake
- Ninja
- Qt 6 development packages for `Core`, `Gui`, and `Widgets`
- Optional Qt DBus development package
- Optional KDE `KF6GlobalAccel` development package if you want KDE global hotkey support

Exact package names depend on your distro.

## Build

## Rust Core Tests

```powershell
cargo test -p universal-stickers-core
```

## Build The Desktop App Manually

From the repository root:

```powershell
cmake -S desktop -B desktop/build
cmake --build desktop/build
```

If Qt is not in a default location, pass `CMAKE_PREFIX_PATH`:

```powershell
cmake -S desktop -B desktop/build -DCMAKE_PREFIX_PATH="C:\path\to\Qt\6.x.x\mingw_64"
cmake --build desktop/build
```

On Linux the same flow applies, using your Qt install prefix:

```bash
cmake -S desktop -B desktop/build -DCMAKE_PREFIX_PATH=/path/to/qt
cmake --build desktop/build
```

The desktop build automatically invokes Cargo for `universal-stickers-ffi` and links the generated Rust static library into the Qt executable.

## Windows Helper Script

On Windows, from a clean checkout:

```powershell
.\run.ps1
```

Build without launching:

```powershell
.\run.ps1 -NoRun
```

What `run.ps1` does:

- bootstraps the repo-local Qt/CMake/MinGW toolchain under `.tools/qt` when it is missing
- ensures the Rust target `x86_64-pc-windows-gnu` is installed
- configures the desktop build with CMake
- falls back to `MinGW Makefiles` if `ninja.exe` is not available
- builds the Qt app
- runs `windeployqt` if available
- launches the app unless `-NoRun` is passed

## Run

After a manual build, the desktop executable is:

- Windows: `desktop/build/universal-stickers.exe`
- Linux: `desktop/build/universal-stickers`
