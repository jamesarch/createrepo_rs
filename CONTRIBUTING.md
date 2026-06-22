# Contributing

Thanks for helping improve `createrepo_rs`.

## Development setup

```bash
git clone https://github.com/artifactx-rs/createrepo_rs.git
cd createrepo_rs
cargo build
cargo test --all-targets
```

The project is Rust 1.76+ and uses vendored dependencies for reproducible packaging builds.

## Before opening a pull request

Run the same checks used by CI:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

If you touch GitHub Actions workflows, also run:

```bash
docker run --rm -v "$PWD:/repo" -w /repo rhysd/actionlint:latest
```

## Contribution guidelines

- Keep changes small and focused.
- Prefer fixes with tests when behavior changes.
- Do not add new dependencies unless they are clearly justified.
- Preserve compatibility with dnf/yum metadata consumers.
- For packaging changes, explain which distro/package manager is affected.

## Commit messages

Use clear, rationale-oriented commit messages. Explain why the change is needed, not just what changed.

## Reporting bugs

Please include:

- `createrepo_rs --version`
- OS/distribution
- command line used
- minimal RPM set or reproducible fixture, if possible
- expected vs actual behavior
