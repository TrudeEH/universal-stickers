#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 || $# -gt 3 ]]; then
    echo "Usage: $0 <built-exe> <output-dir> [zip-path]" >&2
    exit 2
fi

exe_path="$1"
output_dir="$2"
zip_path="${3:-}"

if [[ ! -f "$exe_path" ]]; then
    echo "Windows executable not found: $exe_path" >&2
    echo "Run cargo build --release --locked -p universal-stickers first." >&2
    exit 1
fi

if ! command -v cygpath >/dev/null 2>&1; then
    echo "This packager must run from an MSYS2/MINGW shell so cygpath is available." >&2
    exit 1
fi

if ! command -v ntldd >/dev/null 2>&1; then
    echo "Missing ntldd. Install mingw-w64-x86_64-ntldd in the MSYS2 environment." >&2
    exit 1
fi

if [[ -n "$zip_path" ]] && ! command -v zip >/dev/null 2>&1; then
    echo "Missing zip. Install zip in the MSYS2 environment or omit [zip-path]." >&2
    exit 1
fi

mingw_prefix="${MINGW_PREFIX:-/mingw64}"

rm -rf "$output_dir"
mkdir -p "$output_dir"

cp "$exe_path" "$output_dir/"
cp README.md LICENSE.txt "$output_dir/"

copy_dll_dependencies() {
    local binary="$1"
    # ntldd -R recursively resolves PE/COFF runtime dependencies.  Keep only
    # dependencies that live under the active MINGW prefix so MSYS runtime files
    # and Windows system DLLs are not redistributed accidentally.
    ntldd -R "$binary" \
        | awk -v prefix="$mingw_prefix" '
            $0 ~ /=>/ {
                path = $3
                gsub(/^[[:space:]]+|[[:space:]]+$/, "", path)
                if (index(path, prefix "/bin/") == 1 && path ~ /\.dll$/) {
                    print path
                }
            }
            $0 !~ /=>/ {
                path = $1
                gsub(/^[[:space:]]+|[[:space:]]+$/, "", path)
                if (index(path, prefix "/bin/") == 1 && path ~ /\.dll$/) {
                    print path
                }
            }
        ' \
        | sort -u \
        | while IFS= read -r dll; do
            cp -u "$dll" "$output_dir/"
        done
}

copy_dll_dependencies "$exe_path"

for helper in "$mingw_prefix"/bin/gspawn-*-helper*.exe; do
    if [[ -f "$helper" ]]; then
        cp -u "$helper" "$output_dir/"
    fi
done

# GTK/libadwaita need more than DLLs at runtime.  Bundle the relocatable data
# directories used for icons, themes, GSettings schemas, image loaders, and GTK
# print/media modules so the zip can run on machines without MSYS2 installed.
for dir in \
    "$mingw_prefix/etc/fonts" \
    "$mingw_prefix/etc/gtk-4.0" \
    "$mingw_prefix/lib/gdk-pixbuf-2.0" \
    "$mingw_prefix/lib/girepository-1.0" \
    "$mingw_prefix/lib/gtk-4.0" \
    "$mingw_prefix/share/glib-2.0" \
    "$mingw_prefix/share/icons" \
    "$mingw_prefix/share/libadwaita" \
    "$mingw_prefix/share/themes"; do
    if [[ -d "$dir" ]]; then
        mkdir -p "$output_dir/$(dirname "${dir#$mingw_prefix/}")"
        cp -a "$dir" "$output_dir/${dir#$mingw_prefix/}"
    fi
done

# Loadable GTK and gdk-pixbuf modules can have DLL dependencies that are not in
# the executable's direct closure, so resolve dependencies for bundled modules
# after copying the runtime directories.
while IFS= read -r module; do
    copy_dll_dependencies "$module"
done < <(find "$output_dir/lib" -type f -name '*.dll' 2>/dev/null | sort)

# Fontconfig may rely on a cache directory existing even when it can rebuild it.
mkdir -p "$output_dir/var/cache/fontconfig"

# Include a launcher that sets paths explicitly for users who run from shells or
# launchers that do not preserve GTK's relative lookup behavior.
cat > "$output_dir/Universal Stickers.cmd" <<'CMD'
@echo off
setlocal
set "APP_DIR=%~dp0"
set "PATH=%APP_DIR%;%PATH%"
set "XDG_DATA_DIRS=%APP_DIR%share"
set "GSETTINGS_SCHEMA_DIR=%APP_DIR%share\glib-2.0\schemas"
set "GTK_EXE_PREFIX=%APP_DIR%"
set "GTK_DATA_PREFIX=%APP_DIR%"
start "" "%APP_DIR%universal-stickers.exe" %*
CMD

cat > "$output_dir/WINDOWS-README.txt" <<'TXT'
Universal Stickers for Windows
==============================

This folder is self-contained. Keep the EXE, DLLs, and the bundled etc/, lib/,
share/, and var/ folders together.

Start the app by double-clicking either:

- Universal Stickers.cmd (recommended)
- universal-stickers.exe

The CMD launcher sets GTK and GSettings paths explicitly for Windows installs
that do not have MSYS2, GTK, or libadwaita installed system-wide.
TXT

if [[ -n "$zip_path" ]]; then
    rm -f "$zip_path"
    mkdir -p "$(dirname "$zip_path")"
    zip_abs="$(cd "$(dirname "$zip_path")" && pwd -P)/$(basename "$zip_path")"
    (
        cd "$(dirname "$output_dir")"
        zip -r "$(cygpath -u "$zip_abs")" "$(basename "$output_dir")"
    )
fi
