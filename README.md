# cargo-depsize

<p align="center">
  <img src="https://raw.githubusercontent.com/Alfex4936/cargo-depsize/main/logo.png" alt="cargo-depsize logo" width="180">
</p>

Report the source size of your project’s direct Cargo dependencies.

## Install

Requires Rust and Cargo 1.88 or newer.

```sh
cargo install cargo-depsize --locked
```

To build and install the current checkout, run `cargo install --path . --locked`.

## Usage

Run from a Cargo package or workspace:

```sh
cargo depsize
```

```text
example-crate (v1.2.3)    : 120.00KB (122880 bytes)
other-crate (v0.4.0)      : 256.00KB (262144 bytes)
> Total size: 376.00KB (385024 bytes)
```

| Option | Behavior |
| --- | --- |
| `--manifest-path <PATH>` | Inspect the given `Cargo.toml`. |
| `--locked` | Require the existing lockfile to be up to date. |
| `--offline` | Resolve using only dependencies already available locally. |
| `--help` | Show all options. |

## What the report counts

- Each resolved **direct normal dependency**, including optional dependencies with all features enabled and normal dependencies declared for other targets. Build-only, dev-only, and transitive-only dependencies are excluded.
- A package directory's regular files, with `.ignore`, `.gitignore`, and standard hidden-file rules applied. The scanner skips symlinks, package-local `target/` directories, and the workspace’s configured Cargo output directories. The byte counts describe the local source tree.
- At a virtual workspace root, the report combines direct dependencies of the default members. A package shared by multiple members is counted once. From a member directory, the report uses that member's dependencies.

Cargo may download dependencies during resolution. Use `--locked --offline` when you need to inspect an already cached project without modifying its lockfile.

## Development

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
```

Licensed under [MIT](LICENSE).
