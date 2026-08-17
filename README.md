# Localsend Cli

An ultra light cli localsend client written in Rust. (~1M binary size)


## Badges

[![GPLv3 License](https://img.shields.io/badge/License-GPL%20v3-yellow.svg)](https://opensource.org/licenses/)

![Version number](https://img.shields.io/badge/Version-v1.0.0-blue)


## Appendix

> ⚠️ **Ecryption mode mustn't be enabled** in the official client if you want to transfer files with this project and the official client.


## Features

- Official client compatibility
- Ultra light
- Receive and send files in the local net(http only)
- Cross platform


## Installation

- Option 1: Install pre-built binary via github release

- Option 2: Install via Cargo (For Rust Developers)

```bash
git clone https://github.com/c9ac/localsend-cli.git
cd localsend-cli
cargo build --release
```


## Quick Start

- Receive files

```bash
localsend-cli -n my_device receive  # Use `my_device` as name
```

- Send files

```bash
localsend-cli send -f <path_to_file_a> -f <path_to_file_b> -t 10  # Set timeout for searching devices to 10s
```


## Roadmap

- [x] Receiving and sending files
- [ ] Async I/O support for better performance
- [ ] TUI for interactive sessions
