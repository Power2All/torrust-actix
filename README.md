# Torrust-Axum Tracker
![Test](https://github.com/Power2All/torrust-axum/actions/workflows/rust.yml/badge.svg)

## Project Description
Torrust-Axum Tracker is a lightweight but incredibly powerful and feature-rich BitTorrent Tracker made using Rust.

Currently, it's being actively used at https://www.gbitt.info/ which as of current writing has 100 million torrent hashes loaded and hitting 5 million peers.

This project originated from Torrust Tracker code originally developed by Mick van Dijke, further developed by Power2All as alternative for OpenTracker and other tracker code available on GitHub.

### Features
* [X] Multiple UDP server and HTTP(S) server blocks for socket binding possibilities
* [X] Full IPv4 and IPv6 support for both UDP and HTTP(S)
* [X] Built-in API on a separate port in HTTP
* [X] Persistence saving supported using SQLite3, MySQL or PostgreSQL database
* [X] Customize table and database names in the configuration file for persistence
* [X] Whitelist system, which can be used to make the tracker private
* [X] Blacklist system, to block and ban hashes
* [X] Web Interface (through API) to control the tracker software
* [X] Torrent key support, for private tracking support
* [ ] Dockerfile to build an image for Docker

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
* Running the code will create a `config.toml` file when it doesn't exist yet. The configuration will be filled with default values, and will use SQLite3 in memory as default persistence. Persistence is turned OFF by default, so you need to activate that manually:
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
persistence = false
persistence_interval = 60
api_key = "MyAccessToken"
whitelist = false
blacklist = false
keys = true
keys_cleanup_interval = 10
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

[db_structure]
db_torrents = "torrents"
table_torrents_info_hash = "info_hash"
table_torrents_completed = "completed"
db_whitelist = "whitelist"
table_whitelist_info_hash = "info_hash"
db_blacklist = "blacklist"
table_blacklist_info_hash = "info_hash"
db_keys = "keys"
table_keys_hash = "hash"
table_keys_timeout = "timeout"
```

* Run the torrust-axum again after finishing the configuration:
```bash
./target/release/torrust-axum
```

### Tracker URL
Your tracker announce URL will be the following, depending on what blocks you have enabled:
* `udp://127.0.0.1:6969/announce`
* `http://127.0.0.1:6969/announce`
* `https://127.0.0.1:6969/announce`

#### When Keys system is enabled, following announce URLs should be used:

* `udp://127.0.0.1:6969/announce/1234567890123456789012345678901234567890`
* `http://127.0.0.1:6969/announce/1234567890123456789012345678901234567890`
* `https://127.0.0.1:6969/announce/1234567890123456789012345678901234567890`

### Built-in API
The following URLs are available if you have enabled the API block.
Also, the following URL is enabled for the Web Interface: `http(s)://127.0.0.1:8080/webgui/`
Replace ``[TOKENID]`` with the token set in the configuration file.
Replace ``[TORRENT_HASH]`` with a hex 40 character info_hash.
Also depends on if you have HTTP and/or HTTPS enabled.
If a error occurred for whatever reason, the status key will not contain "ok", but the reason:

```json
{
  "status":"FAILURE REASON"
}
```

#### GET `http(s)://127.0.0.1:8080/api/stats?token=[TOKENID]`
This will show statistics of the tracker in JSON format.

```json
{
  "started":1234567890,
  "timestamp_run_save":1234567890,
  "timestamp_run_timeout":1234567890,
  "timestamp_run_console":1234567890,
  "torrents":0,
  "torrents_updates":0,
  "torrents_shadow":0,
  "seeds":0,
  "peers":0,
  "completed":0,
  "whitelist_enabled":true,
  "whitelist":0,
  "blacklist_enabled":true,
  "blacklist":0,
  "keys_enabled":true,
  "keys":0,
  "tcp4_connections_handled":0,
  "tcp4_api_handled":0,
  "tcp4_announces_handled":0,
  "tcp4_scrapes_handled":0,
  "tcp6_connections_handled":0,
  "tcp6_api_handled":0,
  "tcp6_announces_handled":0,
  "tcp6_scrapes_handled":0,
  "udp4_connections_handled":0,
  "udp4_announces_handled":0,
  "udp4_scrapes_handled":0,
  "udp6_connections_handled":0,
  "udp6_announces_handled":0,
  "udp6_scrapes_handled":0
}
```

#### GET `http(s)://127.0.0.1:8080/api/torrent/[TORRENT_HASH]?token=[TOKENID]`
This will show the content of the torrent, including peers.

```json
{
  "info_hash":"1234567890123456789012345678901234567890",
  "completed":0,
  "seeders":1,
  "leechers":0,
  "peers": [
    [
      {
        "client":"",
        "id":"1234567890123456789012345678901234567890"
      },
      {
        "downloaded":0,
        "event":"Started",
        "ip":"127.0.0.1:1234",
        "left":0,
        "updated":0,
        "uploaded":0
      }
    ]
  ]
}
```

#### DELETE `http(s)://127.0.0.1:8080/api/torrent/[TORRENT_HASH]?token=[TOKENID]`
This will remove the torrent and it's peers from the memory.

```json
{
  "status":"ok"
}
```

#### GET `http(s)://127.0.0.1:8080/api/whitelist?token=[TOKENID]`
This will get the whole whitelist in list format.

```json
[
  "1234567890123456789012345678901234567890",
  "0987654321098765432109876543210987654321"
]
```

#### GET `http(s)://127.0.0.1:8080/api/whitelist/[TORRENT_HASH]?token=[TOKENID]`
This will check if an info_hash exists in the whitelist, and returns if true.

```json
{
  "status":"ok"
}
```

#### POST `http(s)://127.0.0.1:8080/api/whitelist/[TORRENT_HASH]?token=[TOKENID]`
This will insert an info_hash in the whitelist, and returns status if successful.

```json
{
  "status":"ok"
}
```

#### DELETE `http(s)://127.0.0.1:8080/api/whitelist/[TORRENT_HASH]?token=[TOKENID]`
This will remove an info_hash from the whitelist, and returns status if successful or failure reason.

```json
{
  "status":"ok"
}
```

#### GET `http(s)://127.0.0.1:8080/api/blacklist?token=[TOKENID]`
This will get the whole blacklist in list format.

```json
[
  "1234567890123456789012345678901234567890",
  "0987654321098765432109876543210987654321"
]
```

#### GET `http(s)://127.0.0.1:8080/api/blacklist/[TORRENT_HASH]?token=[TOKENID]`
This will check if an info_hash exists in the blacklist, and returns if true.

```json
{
  "status":"ok"
}
```

#### POST `http(s)://127.0.0.1:8080/api/blacklist/[TORRENT_HASH]?token=[TOKENID]`
This will insert an info_hash in the blacklist, and returns status if successful.

```json
{
  "status":"ok"
}
```

#### DELETE `http(s)://127.0.0.1:8080/api/blacklist/[TORRENT_HASH]?token=[TOKENID]`
This will remove an info_hash from the blacklist, and returns status if successful or failure reason.

```json
{
  "status":"ok"
}
```

#### GET `http(s)://127.0.0.1:8080/api/keys?token=[TOKENID]`
This will get the whole keys in list format. 1st value is the key itself, 2nd value is the timestamp in UNIX format (seconds).

```json
[
  [
    "1234567890123456789012345678901234567890",
    "1234567890"
  ]
]
```

#### GET `http(s)://127.0.0.1:8080/api/keys/[KEY]?token=[TOKENID]`
This will check if a key exists in the keys list, and returns if true.

```json
{
  "status":"ok"
}
```

#### POST `http(s)://127.0.0.1:8080/api/keys/[KEY]/[TIMEOUT]?token=[TOKENID]`
This will insert or update a key in the keys list, and returns status if successful. The `[TIMEOUT]` is a number in seconds. Make this 0 to keep the key permanent.

```json
{
  "status":"ok"
}
```

#### DELETE `http(s)://127.0.0.1:8080/api/keys/[KEY]?token=[TOKENID]`
This will remove a key from the keys list, and returns status if successful or failure reason.

```json
{
  "status":"ok"
}
```

### Credits
This Torrust-Tracker was a joint effort by [Nautilus Cyberneering GmbH](https://nautilus-cyberneering.de/), [Dutch Bits](https://dutchbits.nl) and [Power2All](https://power2all.com).
Also thanks to [Naim A.](https://github.com/naim94a/udpt) and [greatest-ape](https://github.com/greatest-ape/aquatic) for some parts in the Torrust-Tracker code.
This project (Torrust-Axum) is built from scratch by [Power2All](https://power2all.com).