# Packaging

Distribution packaging files for createrepo_rs.

## Strategy

| Distro | Priority | Effort | Impact | Status |
|--------|----------|--------|--------|--------|
| Fedora COPR | P0 | Low | RPM ecosystem home | Spec ready |
| Arch AUR | P0 | Low | Easy entry, good visibility | PKGBUILD ready |
| Homebrew | P1 | Low | macOS devs, CI/CD | Formula ready |
| Debian/Ubuntu | P1 | Medium | ~40% Linux market share | debian/ ready |
| Nix/NixOS | P1 | Low | Growing community, reproducible | flake ready |
| Gentoo | P2 | Low | Source-based distro | Ebuild ready |
| Fedora Official | P1 | Medium | Official RPM distro | Needs review |
| EPEL | P1 | Medium | RHEL/Alma/Rocky users | Same spec |
| openSUSE OBS | P2 | Low | Multi-distro via one spec | Same spec |

## Per-Distro Guide

### Fedora COPR (P0 — easiest RPM entry)

```bash
# Install copr CLI
sudo dnf install copr-cli

# Create a COPR project (one-time)
copr-cli create createrepo-rs \
  --description "Pure Rust RPM repository metadata generator" \
  --chroot fedora-rawhide-x86_64 \
  --chroot fedora-41-x86_64 \
  --chroot epel-9-x86_64

# Build from spec
copr-cli build createrepo-rs packaging/rpm/createrepo-rs.spec

# Users install via:
# sudo dnf copr enable yourusername/createrepo-rs
# sudo dnf install createrepo-rs
```

Path to official Fedora: after COPR proves popularity → submit for package review at bugzilla.redhat.com.

### Arch AUR (P0 — immediate)

```bash
# Prepare release tarball and get checksum
# Update sha256sums in PKGBUILD first:
# cd packaging/aur && makepkg -g

# Submit to AUR
git clone ssh://aur@aur.archlinux.org/createrepo-rs.git
cp packaging/aur/PKGBUILD createrepo-rs/
cd createrepo-rs
makepkg --printsrcinfo > .SRCINFO
git add PKGBUILD .SRCINFO
git commit -m "Initial import: createrepo-rs 0.1.8"
git push
```

Users install via: `yay -S createrepo-rs` or `paru -S createrepo-rs`.

### Homebrew (P1 — macOS/CI)

```bash
# First, calculate SHA256 of the release tarball:
# curl -sL https://github.com/artifactx-rs/createrepo_rs/archive/refs/tags/v0.1.8.tar.gz | sha256sum
# Update the sha256 field in packaging/homebrew/createrepo-rs.rb

# Submit to homebrew-core via PR
# Fork homebrew-core, add Formula/c/createrepo-rs.rb, send PR
```

Users install via: `brew install createrepo-rs`.

### Fedora Official (P1 — needs review)

After COPR proves demand, submit for official inclusion:

1. File a "Package Review" bug at bugzilla.redhat.com
2. Use `fedora-review` tool for automated checks
3. Request a sponsor if this is your first Fedora package
4. Once approved, request SCM (dist-git) repo

### EPEL (P1 — Enterprise Linux)

Same spec file works. After Fedora acceptance, request EPEL branches (epel9, epel10).

### openSUSE OBS (P2 — multi-distro for free)

OBS (Open Build Service) can build the same spec for:
- openSUSE Tumbleweed/Leap
- Fedora/RHEL/CentOS
- Debian/Ubuntu (needs separate debian/ packaging)

```bash
# Install osc CLI
# Create OBS project and upload spec
osc checkout home:yourusername
osc mkpac createrepo-rs
cp packaging/rpm/createrepo-rs.spec home:yourusername/createrepo-rs/
osc commit -m "Initial import"
```

### Debian/Ubuntu (P1 — debian/ ready)

```bash
# Build the package
sudo apt install debhelper cargo rustc
dpkg-buildpackage -us -uc -b

# Or use sbuild/pbuilder for clean chroot builds
# Submit to Debian via mentors.debian.net
# Ubuntu: request sync from Debian or upload to PPA

# Users install via:
# sudo apt install ./createrepo-rs_0.1.8-1_amd64.deb
```

For official Debian inclusion:
1. Create an account on mentors.debian.net
2. Upload the package and file an ITP (Intent to Package) bug
3. Find a Debian Developer (DD) to sponsor the upload

### Nix/NixOS (P1 — flake ready)

```bash
# Test build
nix-build -E '(import <nixpkgs> {}).callPackage ./packaging/nix/default.nix {}'

# Using flakes
nix build .#createrepo-rs

# Fill in sha256 and cargoHash after first build attempt:
# 1. Run nix-build, it will fail showing the expected hash
# 2. Copy the hash into default.nix

# Submit to nixpkgs: PR adding pkgs/tools/package-management/createrepo-rs/
```

Users install via: `nix profile install github:artifactx-rs/createrepo_rs` or from nixpkgs after merge.

### Gentoo (P2 — ebuild ready)

```bash
# Install from local overlay
mkdir -p /var/db/repos/local/app-admin/createrepo-rs
cp packaging/gentoo/app-admin/createrepo-rs/* /var/db/repos/local/app-admin/createrepo-rs/
ebuild /var/db/repos/local/app-admin/createrepo-rs/createrepo-rs-0.1.8.ebuild manifest
emerge --ask app-admin/createrepo-rs

# Submit to ::gentoo via PR (or to ::guru for new packages)
# GURU overlay is the easier entry point for new Gentoo packages
```

Users install via: `emerge app-admin/createrepo-rs` (from GURU overlay first, then official ::gentoo).

## Releasing a New Version

1. Update version in all files:
   - `packaging/rpm/createrepo-rs.spec`: Version + changelog
   - `packaging/aur/PKGBUILD`: pkgver + sha256sums
   - `packaging/homebrew/createrepo-rs.rb`: url + sha256
   - `packaging/debian/changelog`: new version entry (`dch -v`)
   - `packaging/nix/default.nix`: version + sha256
   - `packaging/gentoo/.../createrepo-rs-X.Y.Z.ebuild`: new ebuild
2. Push release tag → GitHub Actions builds binary
3. Update COPR / AUR / Homebrew / Debian / Nix / Gentoo
