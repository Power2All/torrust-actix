# Torrust-Axum Tracker
![Test](https://github.com/Power2All/torrust-axum/actions/workflows/rust.yml/badge.svg)

## Project Description
Torrust-Axum Tracker is a lightweight but incredibly powerful and feature-rich BitTorrent Tracker made using Rust.

This project originated from Torrust Tracker code originally developed by Mick van Dijke, further developed by Power2All as alternative for OpenTracker and other tracker code available on GitHub.

### Features
* [X] Multiple UDP server and HTTP(S) server blocks for socket binding possibilities
* [X] Full IPv4 and IPv6 support for both UDP and HTTP(S)
* [X] Built-in API on a separate port in HTTP
* [X] Persistency saving supported using SQLite3, MySQL or PostgreSQL database

### Implemented BEPs
* [BEP 3](https://www.bittorrent.org/beps/bep_0003.html): The BitTorrent Protocol
* [BEP 7](https://www.bittorrent.org/beps/bep_0007.html): IPv6 Support
* [BEP 15](http://www.bittorrent.org/beps/bep_0015.html): UDP Tracker Protocol for BitTorrent
* [BEP 23](http://bittorrent.org/beps/bep_0023.html): Tracker Returns Compact Peer Lists
* [BEP 48](http://bittorrent.org/beps/bep_0048.html): Tracker Protocol Extension: Scrape

## Getting Started
You can get the latest binaries from [releases](https://github.com/Power2All/torrust-axum/releases) or follow the install from scratch instructions below.

### Install From Scratch
1. Clone the repository:
```bash
git clone https://github.com/Power2All/torrust-axum.git
cd torrust-axum
```

2. Build the source code using Rust (make sure you have installed rustup with stable branch)
```bash
cargo build --release
```

### Usage
* Running the code will create a `config.toml` file when it doesn't exist yet. The configuration will be filled with default values, and will use SQLite3 in memory as default persistency. Persistency is turned OFF by default, so you need to activate that manually:
```bash
./target/release/torrust-axum
```

* Modify the newly created `config.toml` file according to your liking. (ToDo: Create extended documentation)
```toml
log_level = "info"
log_console_interval = 60
statistics_enabled = true
db_driver = "SQLite3"
db_path = "sqlite://:memory:"
persistency = false
persistency_interval = 60
interval = 1800
interval_minimum = 1800
interval_cleanup = 900
peer_timeout = 2700
peers_returned = 200

[[udp_server]]
enabled = true
bind_address = "127.0.0.1:6969"

[[http_server]]
enabled = true
bind_address = "127.0.0.1:6969"
ssl = false
ssl_key = ""
ssl_cert = ""

[[api_server]]
enabled = true
bind_address = "127.0.0.1:8080"
ssl = false
ssl_key = ""
ssl_cert = ""
```

* Run the torrust-axum again after finishing the configuration:
```bash
./target/release/torrust-axum
```

### Tracker URL
Your tracker announce URL will be the following, depending what blocks you have enabled:
* `udp://127.0.0.1:6969/announce`
* `http://127.0.0.1:6969/announce`
* `https://127.0.0.1:6969/announce`

### Built-in API
The following URL's are available if you have enabled the API block:
* `http://127.0.0.1:8080/stats` - This will show statistics of the tracker in JSON format.

### Credits
This Torrust-Tracker was a joint effort by [Nautilus Cyberneering GmbH](https://nautilus-cyberneering.de/), [Dutch Bits](https://dutchbits.nl) and [Power2All](https://power2all.com).
Also thanks to [Naim A.](https://github.com/naim94a/udpt) and [greatest-ape](https://github.com/greatest-ape/aquatic) for some parts in the Torrust-Tracker code.
This project (Torrust-Axum) is built from scratch by [Power2All](https://power2all.com).