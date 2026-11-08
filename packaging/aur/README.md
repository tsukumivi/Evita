# AUR binary recipe

This directory prepares `evita-bin` for an AUR maintainer. It is **not an AUR
publication**, and no AUR package availability or ownership is claimed. The recipe
repackages the project's existing, voice-enabled x86_64 Arch release without
compiling Rust or configuring an additional pacman repository. Evita does not yet
have a GitHub Release, so the current source URL and checksum are a migration
template and must be refreshed from the first Evita Arch release before an AUR
submission.
The native package's dependency version floors, desktop integration, icons, notices
and bundled licenses are preserved. ARM Arch binaries are not provided.

Review `PKGBUILD`, then run on an up-to-date Arch host as an unprivileged user:

```sh
cd packaging/aur
makepkg --verifysource
bash check-dependencies.sh
makepkg -si
```

These are owner installation instructions; automated validation never installs
or launches Evita. `evita-bin` conflicts with the repository/source `evita`
package and a future `evita-git` variant because they install the same executable.
Pacman handles a deliberate replacement. Account caches and saved credentials
are unaffected by package removal; see the normal storage policy.

The AUR maintainer should review the source URL/hash and synchronize dependencies
from the verified Arch release's `.PKGINFO` whenever updating `pkgver`. Keep the
`pre.nightly` version conversion used by native Arch packaging; do not put `-`
in `pkgver`, and never replace the checksum with `SKIP`. Generate and review
`.SRCINFO` with `makepkg --printsrcinfo > .SRCINFO` before submitting. Publishing
requires a maintainer-controlled AUR account and package name; this repository's
GitHub push/release automation does not publish to the AUR.

`check-dependencies.sh` compares the package names in the downloaded release's
`.PKGINFO` with the recipe. It ignores version-floor differences because Arch is
rolling and CI's package database can advance between AUR template updates. Keep
the recipe's ABI floors synchronized with the verified release when updating
`pkgver`; CI still rejects missing or extra runtime package names.

[Arch's PKGBUILD reference](https://man.archlinux.org/man/PKGBUILD.5.en) documents
checksum, metadata and package-function behavior. Native Arch makepkg validation
runs with the Linux packaging job; desktop startup, media devices and live Discord
interoperability still need owner testing.
