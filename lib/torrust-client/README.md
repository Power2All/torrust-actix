# torrust-client

Desktop GUI front-end for torrust-actix, built with the [Slint](https://slint.dev/) UI framework.

## Prerequisites

`torrust-client` uses Slint which requires `fontconfig` on Linux.

```bash
# Debian / Ubuntu
sudo apt-get install libfontconfig1-dev

# Fedora / RHEL
sudo dnf install fontconfig-devel

# Arch
sudo pacman -S fontconfig
```

macOS and Windows do not require additional system libraries.

## Building

This crate is a workspace member but is **excluded from the default build** to avoid
requiring GUI dependencies on headless servers and CI environments.

```bash
# From the repository root — build only this crate
cargo build --release -p torrust-client

# Build the full workspace including this crate
cargo build --release --workspace
```

Alternatively, build it stand-alone:

```bash
cd lib/torrust-client
cargo build --release
```

## Running

```bash
./target/release/torrust-client
```
