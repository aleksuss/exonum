# Soak Tests for Exonum Nodes

This is an internal crate for subjecting Exonum networks to [soak tests].

## Contents

The crate exposes the following binaries:

- [`toggle`](src/toggle.rs). Tests repeatedly switching a service on and off.
  The service generates transactions in the `after_commit` hook.

## Usage

Run the selected binary like this:

```sh
cargo run -p exonum-soak-tests --bin $binary
```

Use `--help` option to find out command-line options specific to the binary.

You may want to set up `RUST_LOG` env variable to check events in the nodes
and/or the core library, for example, `RUST_LOG=exonum_node=info,warn`.

[soak tests]: https://en.wikipedia.org/wiki/Soak_testing