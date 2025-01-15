# cargo-interactive-update

[![Build status](https://github.com/BenJeau/cargo-interactive-update/actions/workflows/release.yaml/badge.svg)](https://github.com/BenJeau/cargo-interactive-update/actions/workflows/release.yaml)
[![Crates.io Version](https://img.shields.io/crates/v/cargo-interactive-update.svg)](https://crates.io/crates/cargo-interactive-update)

Update your direct dependencies interactively to the latest version via crates.io, similar to `pnpm update --interactive --latest` from the JS ecosystem.

## Installation

Install the cargo extension by installing it from crates.io:

```bash
cargo install cargo-interactive-update
```

## Usage

Run the cargo extension:

```bash
cargo interactive-update
```

It will then parse the `Cargo.toml` file to get the direct dependencies and check them via the crates.io API.

It extracts dependencies from the `dependencies`, `dev-dependencies`, `build-dependencies` and `workspace.dependencies` sections and updates only the related sections.

If there are outdated dependencies, it will display them and let you select which ones to update, similar to the following:

```
7 out of the 10 direct dependencies are outdated

Dependencies (1 selected):
● crossterm    2024-08-01 0.28.0  -> 2024-08-01 0.28.1   https://github.com/crossterm-rs/crossterm - A crossplatform terminal library for manipulating terminals.
○ curl         2022-07-22 0.4.44  -> 2024-09-30 0.4.47   https://github.com/alexcrichton/curl-rust - Rust bindings to libcurl for making HTTP requests
○ semver       2024-02-19 1.0.22  -> 2024-05-07 1.0.23   https://github.com/dtolnay/semver - Parser and evaluator for Cargo's flavor of Semantic Versioni
○ serde_json   2024-08-23 1.0.127 -> 2024-09-04 1.0.128  https://github.com/serde-rs/json - A JSON serialization file format

Dev dependencies (1 selected):
● assert_cmd   2023-04-13 2.0.11  -> 2024-08-09 2.0.16   https://github.com/assert-rs/assert_cmd.git - Test CLI Applications.

Build dependencies (0 selected):
○ tonic-build  2022-11-29 0.8.3   -> 2024-09-26 0.12.3   https://github.com/hyperium/tonic - Codegen module of `tonic` gRPC implementation.

Workspace dependencies (1 selected):
● tonic        2022-11-28 0.8.3   -> 2024-09-26 0.12.3   https://github.com/hyperium/tonic - A gRPC over HTTP/2 implementation focused on high performanc


Use arrow keys to navigate, <a> to select all, <i> to invert, <space> to select/deselect, <enter> to update, <esc>/<q> to exit
```

After selecting the dependencies to update, it will run update the `Cargo.toml` file and run `cargo check` if you haven't disabled it via the `--no-check` flag.

## Arguments

- `-a` or `--all`: Selects all dependencies to be updated
- `-y` or `--yes`: Execute without asking for confirmation
- `-n` or `--no-check`: Don't run `cargo check` after updating
- `-p` or `--pin`: Pin dependencies to exact versions, with an `=` prefix

For example, if you want to update all dependencies without asking for confirmation, you can run:

```bash
cargo interactive-update -ay
```

or using the long form:

```bash
cargo interactive-update --all --yes
```

## Development

After cloning the repository, you can install the extension locally with the following command:

```bash
cargo install --path .
```

Afterwards, you can run the extension with the following command:

```bash
cargo interactive-update
```

## License

This project is licensed under the MIT license.
