# server-rs

An experimental Rust continuation of [localzet/Server](https://github.com/localzet/Server). It is a partial port, not a compatible replacement for the PHP library. Both repositories belong to the [localzet-server](https://github.com/topics/localzet-server) family.

[Документация на русском](README.ru.md)

## Build and run

Rust 1.98+ is the currently checked toolchain. The crate uses Rust 2024 and dependencies listed in `Cargo.toml`; it is not dependency-free.

```sh
cargo test --locked --all-targets
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo fmt -- --check
cargo run --locked -- 127.0.0.1:8080
```

The executable starts an echo server. Keep the loopback binding for local development.

## Implemented components

- TCP connection lifecycle, managed shutdown and connection statistics;
- UDP connections, delayed and recurring timers;
- Text and length-prefixed Frame protocols;
- HTTP parsing and response value objects, Redis wire encoding and decoding;
- WebSocket handshake and frame codec;
- file-backed session storage and session serialization;
- initial event-loop interfaces and compatibility types.

Protocol codecs do not automatically form a complete HTTP/WebSocket service. Tests cover TCP/UDP behavior, timer cancellation, session traversal rejection, basic HTTP framing rejection and protocol examples. They do not establish exhaustive standards conformance, adversarial robustness or load capacity.

## Limitations and next steps

The TCP runtime currently uses one thread per connection. Several platform event adapters share a basic implementation; their names do not imply native epoll, Swoole or Windows IOCP support. Redis/MongoDB session handlers require further implementation and runtime validation. Do not use this prototype as an Internet-facing production server.

Before a release: define supported protocol subsets and limits; test fragmented/coalesced input and resource exhaustion; implement and benchmark a real readiness-based event loop; verify session backends with actual services; stabilize the Rust API. Preserve parity tests against the PHP server where behavior is intended to match. Deployment and crates.io publication are not enabled by CI.

[Rust introduction for PHP developers (Russian)](docs/RUST_FOR_PHP_DEVELOPERS.md). See `LICENSE` for licensing.

## Attribution

Maintainer of Localzet contributions: **Ivan Zorin (localzet)** — <creator@localzet.com> · https://www.localzet.com. Copyright © 2026 Localzet Group. Original authorship and third-party licenses remain applicable. See [AUTHORS](.github/AUTHORS.md).
