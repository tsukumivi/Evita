#!/usr/bin/env bash
# Compare required package names. Arch's rolling versions make ABI floor strings
# differ between the pinned AUR template and the current CI build every few days.
set -euo pipefail
cd "$(dirname "$0")"
source ./PKGBUILD
archive=${1:-${source_x86_64[0]##*/}}
comparison_dir=$(mktemp -d)
trap 'rm -rf "$comparison_dir"' EXIT
printf '%s\n' "${depends[@]}" | sed -E 's/[<=>].*$//' | LC_ALL=C sort -u > "$comparison_dir/recipe"
bsdtar -xOf "$archive" .PKGINFO > "$comparison_dir/pkginfo"
sed -n 's/^depend = //p' "$comparison_dir/pkginfo" \
  | sed -E 's/[<=>].*$//' | LC_ALL=C sort -u > "$comparison_dir/release"
test -s "$comparison_dir/release"
diff -u "$comparison_dir/release" "$comparison_dir/recipe"
