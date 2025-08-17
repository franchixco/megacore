
# MegaCore

MegaCore is the Rust-based backend for MegaBasterd, a cross-platform MEGA downloader/uploader/streaming suite.

This project is a complete rewrite of the original Java-based backend, with a focus on performance, security, and a clean, modern architecture. The goal is to create a headless, backend-focused library that can be consumed by different frontends, with the first target being a new Swift/SwiftUI application.

## Features

*   **High-performance, concurrent downloads:** MegaCore uses an asynchronous, chunk-based approach to download files, allowing for multiple parallel connections to maximize download speed.
*   **Robust error handling and retries:** The download process is designed to be resilient to network errors, with automatic retries and exponential backoff.
*   **Strong cryptography:** MegaCore uses the same cryptographic protocols as the official MEGA clients to ensure the security and privacy of your data.
*   **Modern, modular architecture:** The codebase is organized into a set of small, focused modules, making it easy to understand, maintain, and extend.

## Roadmap

The migration from Java to Rust is being tracked in the [RUST_MIGRATION_ROADMAP.md](RUST_MIGRATION_ROADMAP.md) file.

## Getting Started

To build the project, you will need to have the Rust toolchain installed. You can find instructions for installing Rust [here](https://www.rust-lang.org/tools/install).

Once you have Rust installed, you can build the project by running the following command in the `megacore` directory:

```
cargo build
```

To run the command-line interface, you can use the following command:

```
cargo run -- get <MEGA_URL> -o <OUTPUT_PATH>
```
