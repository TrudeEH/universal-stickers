# Universal Stickers AUR Package

This directory contains the files for the `universal-stickers-git` AUR package.
It builds the app from the upstream Git repository with Cargo and installs the
same desktop metadata used by the Debian and Fedora packages.

## Test Locally On Arch

```bash
cd desktop/packaging/arch/universal-stickers-git
makepkg -si
```

Run `namcap PKGBUILD` and `namcap universal-stickers-git-*.pkg.tar.*` before
publishing if `namcap` is installed.

## Publish To AUR

The AUR stores package recipes, not built binary packages. Publish this package
from an Arch system after adding an AUR SSH key to your AUR account:

```bash
git clone ssh://aur@aur.archlinux.org/universal-stickers-git.git
cp desktop/packaging/arch/universal-stickers-git/PKGBUILD universal-stickers-git/
cd universal-stickers-git
makepkg --printsrcinfo > .SRCINFO
git add PKGBUILD .SRCINFO
git commit -m "Initial import"
git push
```

Regenerate `.SRCINFO` every time `PKGBUILD` metadata changes.

## Stable Package

Use `universal-stickers-git` until the upstream repository has versioned release
tags such as `v0.1.0`. Once those tags exist, add a separate
`universal-stickers` AUR package that builds from a fixed release tag or release
archive instead of the moving Git branch.
