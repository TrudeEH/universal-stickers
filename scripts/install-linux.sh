#!/usr/bin/env bash
set -euo pipefail

prefix="${PREFIX:-/usr}"
destdir="${DESTDIR:-}"
bindir="${destdir}${prefix}/bin"
datadir="${destdir}${prefix}/share"
build_profile="${BUILD_PROFILE:-release}"
target_dir="${CARGO_TARGET_DIR:-target}"
binary="${target_dir}/${build_profile}/universal-stickers"

if [[ ! -x "$binary" ]]; then
    echo "Missing built binary: $binary" >&2
    echo "Run cargo build --release -p universal-stickers first." >&2
    exit 1
fi

install -Dm755 "$binary" "${bindir}/universal-stickers"
install -Dm644 desktop/packaging/dev.trude.UniversalStickers.desktop \
    "${datadir}/applications/dev.trude.UniversalStickers.desktop"
install -Dm644 desktop/packaging/dev.trude.UniversalStickers.metainfo.xml \
    "${datadir}/metainfo/dev.trude.UniversalStickers.metainfo.xml"
install -Dm644 icon.svg \
    "${datadir}/icons/hicolor/scalable/apps/dev.trude.UniversalStickers.svg"
install -Dm644 README.md "${datadir}/doc/universal-stickers/README.md"
install -Dm644 LICENSE.txt "${datadir}/doc/universal-stickers/LICENSE.txt"
