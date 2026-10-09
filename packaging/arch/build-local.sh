#!/usr/bin/env bash
# Builds the Arch package from the current commit (not from a GitHub release):
#   packaging/arch/build-local.sh        → packaging/arch/out/oche-<ver>-<rel>-x86_64.pkg.tar.zst
#   sudo pacman -U packaging/arch/out/oche-*.pkg.tar.zst
# Uncommitted changes are not included.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../.." && pwd)"
ver="$(sed -n 's/^pkgver=//p' "$here/PKGBUILD")"
out="$here/out"

if [[ -n "$(git -C "$repo" status --porcelain)" ]]; then
  echo "warning: uncommitted changes are not part of the package" >&2
fi

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
cp "$here/PKGBUILD" "$work/"
# makepkg uses a source file that is already present instead of downloading it.
git -C "$repo" archive --format=tar.gz --prefix="oche-$ver/" -o "$work/oche-$ver.tar.gz" HEAD

flags=(-f --skipchecksums)
# Node from mise/nvm/asdf is not a pacman package; makepkg would refuse to build without it.
if ! pacman -Q nodejs npm >/dev/null 2>&1 && command -v node >/dev/null && command -v npm >/dev/null; then
  echo "note: using node from PATH ($(command -v node)), skipping makepkg's dependency check" >&2
  flags+=(--nodeps)
fi

mkdir -p "$out"
(cd "$work" && PKGDEST="$out" makepkg "${flags[@]}" "$@")
ls -1 "$out"/oche-*.pkg.tar.zst
