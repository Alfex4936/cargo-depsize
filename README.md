# `cargo-depsize`
===============

`cargo-depsize` is a Rust cargo utility that calculates and displays the total size of each dependency in your Rust project.

# Installation
------------

Install `cargo-depsize` using the following command:

```sh
cargo install cargo-depsize
```

# Usage
-----

After installation, simply run the following command in your Rust project directory:

```sh
cargo depsize
```

This command will display the size of each dependency package in your Rust project, as well as the total size of all dependencies.

# Example Output
--------------

### Cargo.toml

```toml
[dependencies]
actix-rt = "2"
actix-http = "3"
actix-web = "4"
actix-cors = "0.6"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_derive = "1.0"
reqwest = { version = "0.11", features = ["json", "blocking"] }
scraper = "0.15"
chrono = "0.4"
kakao-rs = "0.3"
rand = "0.8"
lazy_static = "1.4.0"
mongodb = "2"
futures = "0.3"
```

### cargo depsize

```python
actix-cors (v0.6.4)       : 120.79KB (123690 bytes)
actix-http (v3.3.1)       : 768.80KB (787252 bytes)
actix-rt (v2.8.0)         : 71.69KB (73408 bytes)
actix-web (v4.3.1)        : 1.00MB (1049274 bytes)
chrono (v0.4.24)          : 985.68KB (1009338 bytes)
futures (v0.3.27)         : 303.00KB (310269 bytes)
kakao-rs (v0.3.4)         : 78.91KB (80803 bytes)
lazy_static (v1.4.0)      : 29.38KB (30081 bytes)
mongodb (v2.4.0)          : 4.27MB (4473014 bytes)
rand (v0.8.5)             : 342.29KB (350509 bytes)
reqwest (v0.11.15)        : 690.98KB (707563 bytes)
scraper (v0.15.0)         : 85.43KB (87484 bytes)
serde (v1.0.158)          : 497.03KB (508959 bytes)
serde_derive (v1.0.158)   : 305.38KB (312706 bytes)
serde_json (v1.0.94)      : 669.40KB (685469 bytes)
> Total size: 10.10MB (10589819 bytes)
```

# Dependencies
------------

`cargo-depsize` is built on top of the following dependencies:

*   [anyhow](https://crates.io/crates/anyhow) - for easy error handling
*   [cargo](https://crates.io/crates/cargo) - for working with Rust workspaces and packages
*   [tokio](https://crates.io/crates/tokio) - for asynchronous file metadata retrieval

# Contributing
------------

Contributions are welcome! If you have any suggestions or improvements, feel free to create a pull request or open an issue on the repository.

# License
-------

`cargo-depsize` is released under the MIT License.
