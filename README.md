# Torrust-Actix Tracker
![Test](https://github.com/Power2All/torrust-actix/actions/workflows/rust.yml/badge.svg)
[<img src="https://img.shields.io/badge/DockerHub-link-blue.svg">](<https://hub.docker.com/r/power2all/torrust-actix>)
[<img src="https://img.shields.io/discord/1476196704163201059?label=Discord">](<https://discord.gg/zMyZJz4U2D>)

## Project Description
Torrust-Actix Tracker is a lightweight but incredibly powerful and feature-rich BitTorrent Tracker made using Rust.

Currently, it's being actively used at https://www.gbitt.info/.

This project originated from Torrust-Tracker code originally developed by Mick van Dijke, further developed by Power2All as alternative for OpenTracker and other tracker code available on GitHub.

## Features
* [X] Block array for TCP tracking (HTTP/HTTPS), UDP tracking and API (HTTP/HTTPS)
* [X] Full IPv4 and IPv6 support
* [X] Persistence saving supported using SQLite3, MySQL or PostgresSQL database
* [X] Customize table and database structure in the configuration
* [X] Whitelist system for private tracking
* [X] Blacklist system for blocking unwelcome hashes
* [X] Torrent key support for locking access to announcement through keys as info_hash with a timeout
* [X] User account support, configurable for also database support
* [X] Swagger UI built-in in the API (toggleable), useful both for testing API and documentation for API
* [X] Sentry SaaS and self-hosted support
* [X] Full Stand-Alone/Master/Slave cluster mode
* [X] Optional Redis/Memcache Caching for peers data (can be used to show on a website for instance, to less burden SQL)
* [X] Cloudflare's "Simple Proxy Protocol" support added (https://developers.cloudflare.com/spectrum/how-to/enable-proxy-protocol/#enable-simple-proxy-protocol-for-udp)
* [X] RtcTorrent implementation (as alternative/replacement for WebTorrent)
* [X] Configurable LZ4/Zstd compression for RTC SDP data (lz4 default, enabled by default)

## Implemented BEPs
* [BEP 3](https://www.bittorrent.org/beps/bep_0003.html): The BitTorrent Protocol
* [BEP 7](https://www.bittorrent.org/beps/bep_0007.html): IPv6 Support
* [BEP 15](https://www.bittorrent.org/beps/bep_0015.html): UDP Tracker Protocol for BitTorrent
* [BEP 23](https://www.bittorrent.org/beps/bep_0023.html): Tracker Returns Compact Peer Lists
* [BEP 41](https://www.bittorrent.org/beps/bep_0041.html): UDP Tracker Protocol Extensions
* [BEP 48](https://www.bittorrent.org/beps/bep_0048.html): Tracker Protocol Extension: Scrape

## Getting Started
You can get the latest binaries from [releases](https://github.com/Power2All/torrust-actix/releases) or follow the install from scratch instructions below.

### Install From Scratch
1. Clone the repository:
```bash
git clone https://github.com/Power2All/torrust-actix.git
cd torrust-actix
```

2. Build the source code using Rust (make sure you have installed rustup with stable branch)
#### Using build script
```bash
cargo build --release
```

> **Note:** `lib/torrust-client` (the optional desktop GUI) is **excluded from the default build**.
> It requires the `fontconfig` system library and a graphical environment (Slint UI framework).
> See [lib/torrust-client — Optional GUI](#optional-gui-torrust-client) below for details.

### Optional GUI: torrust-client

`lib/torrust-client` is a desktop GUI front-end built with the [Slint](https://slint.dev/) UI framework.
It is a workspace member but is **not compiled by default** (`default-members` excludes it).

#### System requirements (Linux)

```bash
# Debian / Ubuntu
sudo apt-get install libfontconfig1-dev

# Fedora / RHEL
sudo dnf install fontconfig-devel

# Arch
sudo pacman -S fontconfig
```

#### Building

```bash
# Build only the GUI client
cargo build --release -p torrust-client

# Or build the entire workspace including the GUI
cargo build --release --workspace
```

### Usage
Run the code using `--help` argument for using in your enironment:
```bash
./target/release/torrust-actix --help
```
Before you can run the server, you need to either have persistence turned off, and when enabled, make sure your database is created and working. See the help argument above how to fix your setup as you wish.

Swagger UI is introduced, and when enabled in the configuration, is accessible through the API via `/swagger-ui/`.

Sentry.io support is introduced, you can enable it in the configuration and the URL where to push the data to.

### Run on Docker
To run this application in Docker, we provided a Docker Compose file that you can use to run it locally or remotely.

You can find it in the "docker" folder called "docker-compose.yml", and in order to deploy it, use the following commands:

```
docker compose build .
docker compose start
```

### Environment Variable Overrides

Use environment variables to override the configuration settings.

```
LOG_LEVEL <off | trace | debug | info | warn | error>
LOG_CONSOLE_INTERVAL <UINT64>

TRACKER__API_KEY <STRING>
TRACKER__WHITELIST_ENABLED <true | false>
TRACKER__BLACKLIST_ENABLED <true | false>
TRACKER__KEYS_ENABLED <true | false>
TRACKER__USERS_ENABLED <true | false>
TRACKER__SWAGGER <true | false>
TRACKER__KEYS_CLEANUP_INTERVAL <UINT64>
TRACKER__REQUEST_INTERVAL <UINT64>
TRACKER__REQUEST_INTERVAL_MINIMUM <UINT64>
TRACKER__PEERS_TIMEOUT <UINT64>
TRACKER__PEERS_CLEANUP_INTERVAL <UINT64>
TRACKER__PEERS_CLEANUP_THREADS <UINT64>
TRACKER__PROMETHEUS_ID <STRING>
TRACKER__RTC_INTERVAL <UINT64>
TRACKER__RTC_PEERS_TIMEOUT <UINT64>
TRACKER__TOTAL_DOWNLOADS <UINT64>
TRACKER__RTC_COMPRESSION_ENABLED <true | false>
TRACKER__RTC_COMPRESSION_ALGORITHM <lz4 | zstd>
TRACKER__RTC_COMPRESSION_LEVEL <UINT64>

TRACKER__CLUSTER <standalone | master | slave>
TRACKER__CLUSTER_ENCODING <binary | json | msgpack>
TRACKER__CLUSTER_TOKEN <STRING>
TRACKER__CLUSTER_BIND_ADDRESS <STRING>
TRACKER__CLUSTER_MASTER_ADDRESS <STRING>
TRACKER__CLUSTER_KEEP_ALIVE <UINT64>
TRACKER__CLUSTER_REQUEST_TIMEOUT <UINT64>
TRACKER__CLUSTER_DISCONNECT_TIMEOUT <UINT64>
TRACKER__CLUSTER_RECONNECT_INTERVAL <UINT64>
TRACKER__CLUSTER_MAX_CONNECTIONS <UINT64>
TRACKER__CLUSTER_THREADS <UINT64>
TRACKER__CLUSTER_SSL <true | false>
TRACKER__CLUSTER_SSL_KEY <STRING>
TRACKER__CLUSTER_SSL_CERT <STRING>
TRACKER__CLUSTER_TLS_CONNECTION_RATE <UINT64>

CACHE__ENABLED <true | false>
CACHE__ENGINE <redis | memcache>
CACHE__ADDRESS <STRING>
CACHE__PREFIX <STRING>
CACHE__TTL <UINT64>

SENTRY__ENABLED <true | false>
SENTRY__DEBUG <true | false>
SENTRY__ATTACH_STACKTRACE <true | false>
SENTRY__SEND_DEFAULT_PII <true | false>
SENTRY__DSN <STRING>
SENTRY__MAX_BREADCRUMBS <UINT64>
SENTRY__SAMPLE_RATE <F32>
SENTRY__TRACES_SAMPLE_RATE <F32>

DATABASE__PERSISTENT <true | false>
DATABASE__INSERT_VACANT <true | false>
DATABASE__REMOVE_ACTION <true | false>
DATABASE__UPDATE_COMPLETED <true | false>
DATABASE__UPDATE_PEERS <true | false>
DATABASE__PATH <STRING>
DATABASE__ENGINE <sqlite3 | mysql | pgsql>
DATABASE__PERSISTENT_INTERVAL <UINT64>

DATABASE_STRUCTURE__TORRENTS__BIN_TYPE_INFOHASH <true | false>
DATABASE_STRUCTURE__TORRENTS__TABLE_NAME <STRING>
DATABASE_STRUCTURE__TORRENTS__COLUMN_INFOHASH <STRING>
DATABASE_STRUCTURE__TORRENTS__COLUMN_SEEDS <STRING>
DATABASE_STRUCTURE__TORRENTS__COLUMN_PEERS <STRING>
DATABASE_STRUCTURE__TORRENTS__COLUMN_COMPLETED <STRING>

DATABASE_STRUCTURE__WHITELIST__BIN_TYPE_INFOHASH <true | false>
DATABASE_STRUCTURE__WHITELIST__TABLE_NAME <STRING>
DATABASE_STRUCTURE__WHITELIST__COLUMN_INFOHASH <STRING>

DATABASE_STRUCTURE__BLACKLIST__BIN_TYPE_INFOHASH <true | false>
DATABASE_STRUCTURE__BLACKLIST__TABLE_NAME <STRING>
DATABASE_STRUCTURE__BLACKLIST__COLUMN_INFOHASH <STRING>

DATABASE_STRUCTURE__KEYS__BIN_TYPE_HASH <true | false>
DATABASE_STRUCTURE__KEYS__TABLE_NAME <STRING>
DATABASE_STRUCTURE__KEYS__COLUMN_HASH <STRING>
DATABASE_STRUCTURE__KEYS__COLUMN_TIMEOUT <STRING>

DATABASE_STRUCTURE__USERS__ID_UUID <true | false>
DATABASE_STRUCTURE__USERS__BIN_TYPE_KEY <true | false>
DATABASE_STRUCTURE__USERS__TABLE_NAME <STRING>
DATABASE_STRUCTURE__USERS__COLUMN_UUID <STRING>
DATABASE_STRUCTURE__USERS__COLUMN_ID <STRING>
DATABASE_STRUCTURE__USERS__COLUMN_ACTIVE <STRING>
DATABASE_STRUCTURE__USERS__COLUMN_KEY <STRING>
DATABASE_STRUCTURE__USERS__COLUMN_UPLOADED <STRING>
DATABASE_STRUCTURE__USERS__COLUMN_DOWNLOADED <STRING>
DATABASE_STRUCTURE__USERS__COLUMN_COMPLETED <STRING>
DATABASE_STRUCTURE__USERS__COLUMN_UPDATED <STRING>

API_0_ENABLED <true | false>
API_0_SSL <true | false>
API_0_BIND_ADDRESS <STRING>
API_0_REAL_IP <STRING>
API_0_SSL_KEY <STRING>
API_0_SSL_CERT <STRING>
API_0_KEEP_ALIVE <UINT64>
API_0_REQUEST_TIMEOUT <UINT64>
API_0_DISCONNECT_TIMEOUT <UINT64>
API_0_MAX_CONNECTIONS <UINT64>
API_0_THREADS <UINT64>
API_0_TLS_CONNECTION_RATE <UINT64>

HTTP_0_ENABLED <true | false>
HTTP_0_SSL <true | false>
HTTP_0_BIND_ADDRESS <STRING>
HTTP_0_REAL_IP <STRING>
HTTP_0_SSL_KEY <STRING>
HTTP_0_SSL_CERT <STRING>
HTTP_0_KEEP_ALIVE <UINT64>
HTTP_0_REQUEST_TIMEOUT <UINT64>
HTTP_0_DISCONNECT_TIMEOUT <UINT64>
HTTP_0_MAX_CONNECTIONS <UINT64>
HTTP_0_THREADS <UINT64>
HTTP_0_TLS_CONNECTION_RATE <UINT64>
HTTP_0_RTCTORRENT = <true | false>

UDP_0_ENABLED <true | false>
UDP_0_BIND_ADDRESS <STRING>
UDP_0_UDP_THREADS <UINT64>
UDP_0_WORKER_THREADS <UINT64>
UDP_0_RECEIVE_BUFFER_SIZE <UINT64>
UDP_0_SEND_BUFFER_SIZE <UINT64>
UDP_0_REUSE_ADDRESS <true | false>
UDP_0_USE_PAYLOAD_IP <true | false>
UDP_0_SIMPLE_PROXY_PROTOCOL <true | false>
```

---

## RtcTorrent — WebRTC BitTorrent in the Browser

RtcTorrent is the built-in WebRTC peer-to-peer library that lets a browser (or Node.js process) act as a BitTorrent seeder or leecher **without any browser plugin or native binary**. It uses the standard HTTP announce endpoint with additional query parameters for WebRTC signalling.

A full protocol white paper is available in [RtcTorrent.md](./RtcTorrent.md).

### How It Works

```
Browser (leecher) ──announce + rtctorrent=1──► Tracker (Torrust-Actix)
                   ◄── SDP offer from seeder ──
Browser ──answer + rtcanswerfor=<peer_id>────► Tracker
Seeder  ──poll (announce) ───────────────────► Tracker
        ◄── SDP answer ──
WebRTC Data Channel established directly between Browser ↔ Seeder
```

### Building the Browser Bundle

```bash
cd lib/rtctorrent
npm install
npm run build          # produces dist/rtctorrent.browser.js (minified)
npm run dev            # watch mode for development
```

The build outputs two bundles:

| File | Target | Use |
|------|--------|-----|
| `dist/rtctorrent.browser.js` | Browser (`<script>`) | Website player/downloader |
| `dist/rtctorrent.node.js` | Node.js (`require`) | CLI seeder, server-side |

### Using the Library on a Website

Copy `dist/rtctorrent.browser.js` to your web server's static assets, then include it in your HTML:

```html
<script src="/assets/rtctorrent.browser.js"></script>
```

#### Downloading a Torrent (Leecher)

```html
<!DOCTYPE html>
<html>
<head><title>RtcTorrent Demo</title></head>
<body>
  <video id="player" controls autoplay style="width:100%"></video>
  <script src="/assets/rtctorrent.browser.js"></script>
  <script>
    const client = new RtcTorrent({
      trackerUrl: 'http://your-tracker.example.com/announce',
      // Optional: override ICE/STUN servers
      iceServers: [
        { urls: 'stun:stun.l.google.com:19302' }
      ]
    });

    // Download via .torrent URL, magnet URI, or parsed torrent object
    client.download('magnet:?xt=urn:btih:INFOHASH&dn=MyVideo&tr=http://your-tracker.example.com/announce')
      .then(torrent => {
        console.log('Started downloading:', torrent.name);
      });

    // Stream a video file directly into a <video> element
    client.streamVideo('INFOHASH_HEX', 0, document.getElementById('player'));
  </script>
</body>
</html>
```

#### Seeding a File from the Browser

```html
<input type="file" id="filePicker">
<script src="/assets/rtctorrent.browser.js"></script>
<script>
  const client = new RtcTorrent({
    trackerUrl: 'http://your-tracker.example.com/announce'
  });

  document.getElementById('filePicker').addEventListener('change', async (e) => {
    const file = e.target.files[0];

    // Create a torrent from the selected file
    const { torrent, magnetUri, infoHash } = await client.create([file], {
      version: 'v1',   // or 'v2' / 'hybrid'
      name: file.name
    });

    console.log('Magnet URI:', magnetUri);
    console.log('Info Hash:', infoHash);

    // Start seeding — the tracker handles WebRTC signalling
    await client.seed(torrent, [file]);
  });
</script>
```

#### Constructor Options

| Option | Default | Description |
|--------|---------|-------------|
| `trackerUrl` | `''` | HTTP announce URL of the Torrust-Actix tracker |
| `announceInterval` | `30000` | Re-announce interval in milliseconds |
| `rtcInterval` | `10000` | WebRTC signalling poll interval in milliseconds |
| `maxPeers` | `50` | Maximum simultaneous WebRTC peers |
| `iceServers` | Google STUN | Array of ICE server objects |

#### Key Methods

| Method | Description |
|--------|-------------|
| `create(files, options)` | Create a torrent from File objects (browser) or file paths (Node) |
| `download(torrentData)` | Download via magnet URI, `.torrent` URL, or parsed torrent object |
| `seed(torrentData, files)` | Seed an existing torrent |
| `streamVideo(infoHash, fileIndex, videoEl)` | Stream a video piece-by-piece into a `<video>` element |
| `stop()` | Stop all torrents and close connections |
| `parseMagnet(uri)` | Parse a magnet URI into a torrent object |
| `parseTorrentFile(buffer)` | Parse a `.torrent` file buffer |
| `calculateInfoHash(info)` | Calculate the SHA-1 info hash of a torrent info dictionary |

### Tracker Configuration for RtcTorrent

Enable RtcTorrent support on the HTTP listener in `config.toml`:

```toml
[[http_trackers]]
enabled = true
bind_address = "0.0.0.0:6969"
rtctorrent = true          # Enable WebRTC signalling endpoint
```

Or via environment variable:
```
HTTP_0_RTCTORRENT=true
```

### CLI Seeder (Node.js)

The `bin/seed.js` script seeds files from the command line:

```bash
# Install dependencies
cd lib/rtctorrent && npm install

# Single file
node bin/seed.js --tracker http://your-tracker.example.com/announce \
                 --name "My Movie" \
                 --out movie.torrent \
                 /path/to/movie.mp4

# Re-seed from an existing .torrent (no re-hashing)
node bin/seed.js --torrent-file movie.torrent /path/to/movie.mp4

# Seed from a magnet URI
node bin/seed.js --magnet "magnet:?xt=urn:btih:..." /path/to/movie.mp4

# Multi-torrent mode via YAML config
node bin/seed.js --torrents torrents.yaml
```

**YAML multi-torrent config example:**

```yaml
torrents:
  - name: "My Movie"
    file:
      - "/data/movie.mp4"
    trackers:
      - "http://your-tracker.example.com/announce"
    out: "/data/movie.torrent"
    version: v1          # v1 | v2 | hybrid
    webseed:
      - "https://cdn.example.com/movie.mp4"
```

Reload the YAML config without restarting by sending `SIGHUP` (Linux/macOS) or simply saving the file (polled every 2 seconds on all platforms).

### Demo

Run the built-in demo server to test locally:

```bash
cd lib/rtctorrent
npm install && npm run build
npm run serve    # serves demo at http://localhost:8080/demo/
```

---

### ChangeLog

#### v4.2.1
* Adding LZ4/ZSTD compression for memory objects for the RtcTorrent peer data, enabled by default, disablable through config
* Memory optimization for the peer data
* Some small bug fixes

#### v4.2.0
* Refactoring the whole project structure
* Adding security and anti-malicious validation and checks
* Adding RtcTorrent library and support to the tracker, for web streaming and downloading of BitTorrent. This is a major change from v4.1.0. It uses the default HTTP BitTorrent announce endpoint, with additional parameters

#### v4.1.1
* Added hot reloading of SSL certificates for renewal
* API has an extra endpoint to run the hot reloading
* Some more code optimizations

#### v4.1.0
* Added a full Cluster first version through WebSockets
* Option to run the app in Stand-Alone (which is default, as single server), or using the cluster mode
* When set to Master, it still functions as if being Stand-Alone, but with WebSocket support for clustering
* When set to Slave, it will forward all the requests to the Master server to be handled
* Added configurations to be applied in config.toml and the environment variables
* Added statistics, also showing cluster statistics next to the rest. Slave will only show active requests
* WebSocket data can be sent in 3 different ways, but Master server is the leading what way to talk
* This is a very early version of the WebSocket cluster implementation, and needs thorough testing
* Moved the more database engines additions to another version (and MeiliSearch/ElasticSearch support for v4.2)
* Refactored the database engine to be less massive, more logical and less redundancy
* Implemented a Redis and Memcache cache optionally to push peer data to, for usage on websites (without burdening SQL)
* Added UDP support for Cloudflare's "Simple Proxy Protocol" (https://developers.cloudflare.com/spectrum/how-to/enable-proxy-protocol/#enable-simple-proxy-protocol-for-udp)

#### v4.0.17
* Another little overhaul, changing some memory tools for enhancement and performance
* Less CPU cycles applied when requests are handled
* Preparing for v4.1 to add some more database engines to the mix apart from SQLx

#### v4.0.16
* Small debug and hotfixes
* Fixed the unit test that somehow failed
* Updated this readme file

#### v4.0.15
* More code optimizations thanks to AI scanning further
* UDP performance tweaks (some new config options added)
* Added an initial unit testing for checking if all functions and features work as expected

#### v4.0.14
* Code optimizations thanks to AI scanning
* Huge memory and CPU consumption improvement for UDP, using offloading

#### v4.0.13
* Added further UDP improvement by adding customization, also added to the config:
  * Receive Buffer Size
  * Send Buffer Size
  * Reuse of Address

#### v4.0.12
* Updating libraries and cleanup code

#### v4.0.11
* Updating libraries
* Adding healthcheck for Docker through Python check script

#### v4.0.10
* Updating libraries
* Adding full environment support to override configurations (Thanks tachyon3000 for the idea)

#### v4.0.9
* Updating libraries (Actix 4.9 to 4.10)
* Some critical exploit in ZIP fixed
* Some faulty v4.0.8 deployments fixed with GitHub

#### v4.0.8
* Updating libraries
* Fixing threading for UDP
* Removed a feature from Parking Lot, cause of an unfixed vulnerability
* Update Swagger UI to version v4.0.0 and OpenAPI version v3.1.1

#### v4.0.7
* Cleanup was still broken, did a big rewrite, after testing it works now as expected
* Did some tokio threading correctly for core threads
* Added a new configuration key, to set the threads, default for each shard (256), but can be changed

#### v4.0.6
* Fixed some clippy issues
* Found a performance issue on peers cleanup
* Switched peers cleanup from Tokio spawn to Thread spawn for speedup
* Bumped version of Tokio

#### v4.0.5
* Library bump

#### v4.0.4
* Further implementation of Sentry (trace logging)

#### v4.0.3
* Fixing announce and scrape paths, since it is the wrong way.
* Fixing various smaller bugs that isn't app-breaking, but should be handled better.
* Added support for Sentry.io SaaS and self-hosted setup.
* Preparing work for version v4.1.0, which will introduce LUA support for middleware.

#### v4.0.2
* Added option that the system will remove data from database.
* Added updates variables for the white/black list and keys tables.
* Renaming the "database" naming which should be "tables".
* A lot of fixes and bugs I stumbled upon.

#### v4.0.0
* Completely rebuilt of the tracker code, for readability.
* Moved to Actix v4, thus versioning this software to v4.0.0 as well.
* Rebuilt and remade the way configuration file is created (you need to give the command as argument for it).
* Redone the whole database system, is tested with the latest versions available at this time.
* API has gone through a lot of work and tested.
* Introduced Swagger UI as testing and documentation.
* A lot of improvements in speed and performance applied further.
* Import and Export function added, will dump or import from JSON files, handy for when making a backup from your existing database, or when migrating to an other database engine.
* Removed WebGUI, was outdated and not really useful.

#### v3.2.2
* Bumped library versions significantly, including security patches.
* Fixed changes in libraries to work properly.
* Tuned the non-persistence code to use less memory.

#### v3.2.1
* Bumped library versions, including security patches.
* Fixed a bug in the PostgreSQL handler.
* Some forgotten naming from Torrust-Axum to Torrust-Actix.

#### v3.2.0
* Bumped library versions.
* Modified the way scheduling was done through threads, it could lock up and slow down public trackers with heavy activity.
* Tweaking the SQLite3 database usage and database space consumption.
* Full overhaul on how torrents and peers are used in memory. Using crossbeam skipmap for thread safe non-locking memory sharing.
* Some various improvement on coding performance, readability and linting the files.
* Replaced Tokio Axum web framework for Actix, reason: Missing critical things like a timeout on connect, disconnect, read and write, and support was lackluster.
* Renamed the GitHub repository from torrust-axum to torrust-actix.
* Adding user tracking support with an extra key.

#### v3.1.2
* Bumped library versions.
* Added a Code of Conduct file, as some open source projects need this.
* Added a Maintenance toggle function to API and WebGUI.
* Configuration file is not generated when it doesn't exist, or has invalid data, unless forced with a '--create-config' argument.
* Fixed various small bugs.

#### v3.1.1
* Bumped library versions.
* Database for SQLite3, MySQL and PostgreSQL now works properly with all the tables, and will be used if enabled.
* UDP had a problem in IPv4, fixed the code for correctly parsing byte array.
* Cleanup and refactoring of some redundant code.
* Added some small checks where needed to prevent errors.

#### v3.1.0
* Whitelist System: You can enable this to only allow torrent hashes to be used you specify in the database, or add them through the API.
* Blacklist System: You can enable this to disallow torrent hashes to be used you specify in the database, or add them through the API.
* Keys System: You can enable this to only allow tracking when an activated "key" hash (same as an info_hash, 20 bytes or 40 characters hex) is given. Keys with a timeout of zero "0" will be permanent and won't be purged by the cleanup.
* WebGUI: The API has an available web interface, which can be accessed through https://your.api:8080/webgui/ and giving the correct API Key, which you configure in the configuration file.
* Customizable database structure can be given in the configuration file.
* The system is also now available through Docker Hub at https://hub.docker.com/r/power2all/torrust-axum

#### v3.0.1
* Bugfixes
* SQLite3 support added
* MySQL support added
* PostgresSQL support added

#### v3.0.0
Initial version of Torrust-Axum.

### Credits
This Torrust-Tracker was a joint effort by [Nautilus Cyberneering GmbH](https://nautilus-cyberneering.de/), [Dutch Bits](https://dutchbits.nl) and [Power2All](https://power2all.com).
Also thanks to [Naim A.](https://github.com/naim94a/udpt) and [greatest-ape](https://github.com/greatest-ape/aquatic) for some parts in the Torrust-Tracker code.
This project (Torrust-Actix) is built from scratch by [Power2All](https://power2all.com).
