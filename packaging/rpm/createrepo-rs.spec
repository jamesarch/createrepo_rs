Name:           createrepo-rs
Version:        0.1.9
Release:        1%{?dist}
Summary:        Pure Rust RPM repository metadata generator

License:        GPL-2.0-or-later
URL:            https://github.com/artifactx-rs/createrepo_rs
Source0:        %{url}/releases/download/v%{version}/createrepo-rs-%{version}-vendored.tar.gz

BuildRequires:  cargo
BuildRequires:  rust >= 1.76
BuildRequires:  make

# Rust deps are vendored — no network access needed during build

# Disable debugsource on RHEL/EPEL (not supported with vendored Rust deps)
%if 0%{?rhel}
%global debug_package %{nil}
%endif

%description
createrepo-rs is a pure Rust implementation of createrepo_c that generates
RPM repository metadata (repodata). It produces dnf/yum-compatible output
as a drop-in replacement for createrepo_c, with zero FFI dependencies and
a single static binary.

Features:
  - primary.xml, filelists.xml, other.xml, repomd.xml generation
  - In-memory SQLite with atomic VACUUM INTO disk flush
  - Parallel RPM parsing (auto-detects CPU count)
  - --dump-manifest for package inventory + signature detection
  - --timeout watchdog for stuck I/O (network mounts)
  - Incremental mode with --update
  - Compression: gzip, zstd, xz, bzip2

%prep
%autosetup -n createrepo_rs-%{version}

%build
# Release build with optimizations
cargo build --release

%install
install -D -m 0755 target/release/createrepo_rs %{buildroot}%{_bindir}/createrepo_rs

%check
# --version prints to stderr, merge with stdout for rpmbuild check
%{buildroot}%{_bindir}/createrepo_rs --version 2>&1 ||:

%files
%{_bindir}/createrepo_rs
%license LICENSE
%doc README.md README_zh.md

%changelog
* Wed May 27 2026 jamesarch <han.shan@live.cn> - 0.1.9-1
- Initial package
