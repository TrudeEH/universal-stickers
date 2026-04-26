# Universal Stickers

Small sticker/GIF picker built with a Rust storage core and a Qt Widgets desktop UI.

The app stores imported images and GIFs in its own library, shows them in a searchable grid, lets you rename or delete them, supports drag-and-drop import, and copies the selected sticker to the clipboard for use in other apps. It also supports export/import backups.

## Project Layout

- `core/`: Rust storage library for import, search, rename, delete, thumbnail generation, and backups.
- `ffi/`: `cxx` bridge crate that exposes the Rust core to the desktop app.
- `desktop/`: Qt Widgets desktop app.
- `run.ps1`: Windows helper script that configures, builds, deploys Qt runtime files, and runs the app.

## Dependencies

## Rust

Install a current Rust toolchain with `cargo` and `rustup`.

This repo uses:

- Rust edition `2024`
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

The helper script expects a repo-local Qt toolchain under `.tools/qt` with this layout:

- `.tools/qt/Tools/CMake_64/bin/cmake.exe`
- `.tools/qt/Tools/mingw1310_64/bin`
- `.tools/qt/6.8.3/mingw_64/bin`

That matches the local build setup currently used by this project.

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

If `.tools/qt` is set up as described above:

```powershell
.\run.ps1
```

Build without launching:

```powershell
.\run.ps1 -NoRun
```

What `run.ps1` does:

- configures the desktop build with CMake + Ninja
- builds the Qt app
- runs `windeployqt` if available
- launches the app unless `-NoRun` is passed

## Run

After a manual build, the desktop executable is:

- Windows: `desktop/build/universal-stickers.exe`
- Linux: `desktop/build/universal-stickers`

## Notes

- The app is now a normal desktop app. It does not use a tray icon or background process.
- On Windows, the current local build uses Qt MinGW, so the Rust GNU target is required.
- On KDE/Linux, the global hotkey is only enabled when `KF6GlobalAccel` is available at build time.
- The build may emit MinGW `.drectve` linker warnings on Windows; the app still links and runs successfully in the current setup.
