# Torrust-Actix Tracker
![Test](https://github.com/Power2All/torrust-actix/actions/workflows/rust.yml/badge.svg)
[<img src="https://img.shields.io/badge/DockerHub-link-blue.svg">](<https://hub.docker.com/r/power2all/torrust-actix>)
[<img src="https://img.shields.io/discord/1476196704163201059?label=Discord">](<https://discord.gg/rC7aEtpwMX>)

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
* [X] Adding UDP receive method with recvmmsg or io_uring

## Implemented BEPs
* [BEP 3](https://www.bittorrent.org/beps/bep_0003.html): The BitTorrent Protocol
* [BEP 7](https://www.bittorrent.org/beps/bep_0007.html): IPv6 Support
* [BEP 15](https://www.bittorrent.org/beps/bep_0015.html): UDP Tracker Protocol for BitTorrent
* [BEP 23](https://www.bittorrent.org/beps/bep_0023.html): Tracker Returns Compact Peer Lists
* [BEP 41](https://www.bittorrent.org/beps/bep_0041.html): UDP Tracker Protocol Extensions
* [BEP 48](https://www.bittorrent.org/beps/bep_0048.html): Tracker Protocol Extension: Scrape

## Getting Started
You can get the latest binaries from [releases](https://github.com/Power2All/torrust-actix/releases) or follow the installation from scratch instructions below.

### Install From Scratch
1. Clone the repository:
```bash
git clone https://github.com/Power2All/torrust-actix.git
cd torrust-actix
```

2. Build the source code using Rust 1.95.0 or newer (make sure you have installed rustup with stable branch)
#### Using build script
```bash
cargo build --release
```

#### System requirements (Linux)

A C compiler (for `ring`, `zstd` and the bundled SQLite) and `curl` (the Swagger UI bundle is downloaded at build time). TLS is rustls throughout, so no OpenSSL is needed:

```bash
# Debian / Ubuntu
sudo apt-get install build-essential curl

# Fedora / RHEL
sudo dnf install gcc make curl

# Arch
sudo pacman -S base-devel curl
```

#### Building

```bash
# Build the project
cargo build --release
```

### Usage
Run the code using `--help` argument for using in your environment:
```bash
./target/release/torrust-actix --help
```
Before you can run the server, you need to either have persistence turned off, and when enabled, make sure your database is created and working. See the help argument above how to fix your setup as you wish.

#### Upgrading a MySQL or PostgreSQL database created before v4.2.24
Databases created with `--create-database` before v4.2.24 store the key expiry (`keys.timeout`) and the user's last update (`users.updated`) as 32-bit Unix timestamps, which cannot hold dates after 19 January 2038. Widen them to 64-bit once, with the tracker stopped:
```bash
./target/release/torrust-actix --migrate-database
```
It reads the table and column names from `config.toml`, is safe to run again, skips tables that do not exist and exits when done. SQLite databases need nothing: SQLite integers are already 64-bit.

Swagger UI is introduced, and when enabled in the configuration, is accessible through the API via `/swagger-ui/`.

Sentry.io support is introduced, you can enable it in the configuration and the URL where to push the data to.

### Run on Docker
To run this application in Docker, we provided a Docker Compose file that you can use to run it locally or remotely.

You can find it in the "docker" folder called "docker-compose.yml". The image is built from the checkout it sits in, and the compose file requires an API key from the environment:

```
cd docker
export TRACKER__API_KEY="$(head -c 32 /dev/urandom | base64)"
docker compose build
docker compose up -d
```

The API is published on `127.0.0.1:8080` only; publish it wider only behind TLS or a firewall. To build the image without compose, run `docker build -f docker/Dockerfile .` from the repository root. The container runs as an unprivileged `tracker` user.

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
TRACKER__TLS_CONNECTION_RATE <UINT64>
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

CACHE__ENABLED <true | false>
CACHE__ENGINE <redis | memcache>
CACHE__ADDRESS <STRING>
CACHE__PREFIX <STRING>
CACHE__TTL <UINT64>
CACHE__SPLIT_PEERS <true | false>

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
DATABASE__CHUNK_SIZE <UINT64>

DATABASE_STRUCTURE__TORRENTS__BIN_TYPE_INFOHASH <true | false>
DATABASE_STRUCTURE__TORRENTS__TABLE_NAME <STRING>
DATABASE_STRUCTURE__TORRENTS__COLUMN_INFOHASH <STRING>
DATABASE_STRUCTURE__TORRENTS__COLUMN_SEEDS <STRING>
DATABASE_STRUCTURE__TORRENTS__COLUMN_PEERS <STRING>
DATABASE_STRUCTURE__TORRENTS__COLUMN_COMPLETED <STRING>
DATABASE_STRUCTURE__TORRENTS__PERSISTENT <true | false>

DATABASE_STRUCTURE__WHITELIST__BIN_TYPE_INFOHASH <true | false>
DATABASE_STRUCTURE__WHITELIST__TABLE_NAME <STRING>
DATABASE_STRUCTURE__WHITELIST__COLUMN_INFOHASH <STRING>
DATABASE_STRUCTURE__WHITELIST__PERSISTENT <true | false>

DATABASE_STRUCTURE__BLACKLIST__BIN_TYPE_INFOHASH <true | false>
DATABASE_STRUCTURE__BLACKLIST__TABLE_NAME <STRING>
DATABASE_STRUCTURE__BLACKLIST__COLUMN_INFOHASH <STRING>
DATABASE_STRUCTURE__BLACKLIST__PERSISTENT <true | false>

DATABASE_STRUCTURE__KEYS__BIN_TYPE_HASH <true | false>
DATABASE_STRUCTURE__KEYS__TABLE_NAME <STRING>
DATABASE_STRUCTURE__KEYS__COLUMN_HASH <STRING>
DATABASE_STRUCTURE__KEYS__COLUMN_TIMEOUT <STRING>
DATABASE_STRUCTURE__KEYS__PERSISTENT <true | false>

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
DATABASE_STRUCTURE__USERS__PERSISTENT <true | false>

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
UDP_0_RECEIVE_METHOD <auto | recvmmsg | io_uring | rio>
UDP_0_PARSE_QUEUE_SIZE <UINT64>
```

#### Database commit chunk size

`database.chunk_size` controls how many rows the tracker writes per SQL transaction when it flushes torrent/user updates to the database (once per `persistent_interval`). Instead of committing the whole sync batch in a single long-running transaction — which holds row/foreign-key locks for the entire batch and can block other applications writing to the same database (e.g. a website doing `DELETE`/`UPDATE`, leading to lock-wait timeouts) — the tracker commits every `chunk_size` rows, releasing locks frequently.

```toml
[database]
chunk_size = 1000   # rows per transaction (default: 1000; 0 = commit the whole batch in one transaction)
```

Lower the value (e.g. `500` or `250`) if another application sharing the database still experiences lock contention; raise it (or set `0`) for maximum write throughput when the tracker owns the database exclusively. Applies to the high-volume `torrents` and `users` sync tables on MySQL, PostgreSQL, and SQLite.

---

## API Authentication

Every `/api/...` endpoint (and `/stats`, `/metrics`) is protected by the API key configured in `tracker.api_key` (or the `TRACKER__API_KEY` environment variable). The key can be supplied in two ways:

1. **`Authorization` header (recommended)** — keeps the secret out of URLs, access logs and browser history:

   ```
   Authorization: Bearer <api_key>
   ```

   The `Bearer ` prefix is optional; a bare `Authorization: <api_key>` is also accepted.

2. **`token` query-string parameter (legacy)** — kept for backwards compatibility:

   ```
   GET /api/torrents?token=<api_key>
   ```

If both are present, the `Authorization` header takes precedence. The key is compared in constant time. Example:

```bash
# Preferred — token in the header
curl -H "Authorization: Bearer <api_key>" http://127.0.0.1:8080/api/torrents

# Legacy — token in the query string
curl "http://127.0.0.1:8080/api/torrents?token=<api_key>"
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

## Caching — Redis & Memcache

The tracker can optionally push live torrent peer statistics, and with `users_enabled` the users' totals, to a Redis or Memcache instance after every database sync cycle.  This lets websites and dashboards read seeder/leecher counts and user stats at high speed without querying the SQL database directly.

### Configuration

Add a `[cache]` section to `config.toml`:

```toml
[cache]
enabled     = true
engine      = "redis"       # "redis" or "memcache"
address     = "127.0.0.1:6379" # or "user:pass@redis:6379", or "redis://:pass@redis:6379/0"
prefix      = "tracker:"    # key namespace prefix
ttl         = 300           # seconds until a torrent key expires (0 = no expiry); user keys never expire
split_peers = false         # see "Key format" below
```

`address` takes a host name or IP and a port, optionally with credentials, or a whole URL with a database number; the password is masked in the logs.

The tracker connects to Redis/Memcache **only when it writes** (once per `persistent_interval` seconds).  No persistent connection is kept open between sync cycles.

> The cache is updated independently of `database.persistent`.  Setting `persistent = false` disables SQL writes but the cache is still updated on every sync cycle.

### Key Format

Every torrent is stored under the key:

```
<prefix>t:<info_hash_hex>
```

For example, with the default prefix `tracker:` and a 40-character hex info hash:

```
tracker:t:a1b2c3d4e5f6...
```

#### Aggregated mode (`split_peers = false`, default)

The key is a Redis hash (or colon-delimited string in Memcache) with **three fields**:

| Field | Type | Description |
|-------|------|-------------|
| `s`   | u64  | Total seeders (BitTorrent IPv4 + IPv6 + WebRTC) |
| `p`   | u64  | Total leechers (BitTorrent IPv4 + IPv6 + WebRTC) |
| `c`   | u64  | Total completed downloads |

**Redis — reading aggregated data:**
```bash
HMGET tracker:t:<info_hash_hex> s p c
# returns: [seeds, peers, completed]
```

**Memcache — reading aggregated data:**
```
GET tracker:t:<info_hash_hex>
# returns: "seeds:peers:completed"  e.g. "42:100:500"
```

#### Split mode (`split_peers = true`)

Each pool and IP version is stored as a separate field, giving a full breakdown:

| Field          | Type | Description |
|----------------|------|-------------|
| `bt_seeds_ipv4` | u64 | BitTorrent IPv4 seeders |
| `bt_seeds_ipv6` | u64 | BitTorrent IPv6 seeders |
| `rtc_seeds`     | u64 | WebRTC seeders |
| `bt_peers_ipv4` | u64 | BitTorrent IPv4 leechers |
| `bt_peers_ipv6` | u64 | BitTorrent IPv6 leechers |
| `rtc_peers`     | u64 | WebRTC leechers |
| `c`             | u64 | Total completed downloads |

**Redis — reading split data:**
```bash
HGETALL tracker:t:<info_hash_hex>
# returns all seven fields as a flat key/value list

# Or fetch specific fields:
HMGET tracker:t:<info_hash_hex> bt_seeds_ipv4 bt_seeds_ipv6 rtc_seeds bt_peers_ipv4 bt_peers_ipv6 rtc_peers c
```

**Memcache — reading split data:**
```
GET tracker:t:<info_hash_hex>
# returns: "bt_seeds_ipv4:bt_seeds_ipv6:rtc_seeds:bt_peers_ipv4:bt_peers_ipv6:rtc_peers:completed"
# e.g.  "30:5:7:80:15:5:500"
#        bt4   bt6 rtc bt4  bt6 rtc completed
```

### PHP example (Redis, aggregated mode)

```php
$redis = new Redis();
$redis->connect('127.0.0.1', 6379);

$infoHash = bin2hex($rawInfoHashBytes); // 40-char hex
$key      = 'tracker:t:' . $infoHash;

[$seeds, $peers, $completed] = $redis->hMGet($key, ['s', 'p', 'c']);

echo "Seeders: $seeds, Leechers: $peers, Completed: $completed";
```

### PHP example (Redis, split mode)

```php
$redis = new Redis();
$redis->connect('127.0.0.1', 6379);

$key  = 'tracker:t:' . $infoHash;
$data = $redis->hGetAll($key);

$totalSeeders  = $data['bt_seeds_ipv4'] + $data['bt_seeds_ipv6'] + $data['rtc_seeds'];
$totalLeechers = $data['bt_peers_ipv4'] + $data['bt_peers_ipv6'] + $data['rtc_peers'];

echo "BT IPv4 seeds: {$data['bt_seeds_ipv4']}";
echo "BT IPv6 seeds: {$data['bt_seeds_ipv6']}";
echo "WebRTC seeds:  {$data['rtc_seeds']}";
```

### User stats

Each user's totals are stored under the key:

```
<prefix>u:<id>
```

`<id>` is the user's row identifier in the users table: the uuid with `id_uuid = true`, otherwise the numeric id. For example `tracker:u:7`.

A user is written whenever it changes, by an announce or through the API, in the same sync cycle as torrents. This does not need a persistent users table: with `persistent = false` the cache is the only place the totals are written. A user deleted through the API is deleted from the cache. Unlike torrent keys, user keys **do not expire** (`ttl` does not apply), because the totals stay valid while the user is not announcing. A user that has not changed since the cache was emptied has no key yet; read it from the users table or the API.

| Field        | Type | Description |
|--------------|------|-------------|
| `uploaded`   | u64  | Total bytes uploaded |
| `downloaded` | u64  | Total bytes downloaded |
| `completed`  | u64  | Total completed downloads |
| `active`     | u64  | `1` when the user may announce, `0` when disabled |
| `updated`    | u64  | Unix time of the user's last announce |
| `torrents`   | u64  | Torrents the user is seeding or leeching |

`uploaded` and `downloaded` are credited when a client sends `stopped`, with that session's totals; `completed` when it sends `completed`.

**Redis:**
```bash
HMGET tracker:u:7 uploaded downloaded completed
```

**Memcache:**
```
GET tracker:u:7
# returns: "uploaded:downloaded:completed:active:updated:torrents"  e.g. "5368709120:300:2:1:1700000000:2"
```

**PHP (Redis):**
```php
[$uploaded, $downloaded] = $redis->hMGet('tracker:u:' . $userId, ['uploaded', 'downloaded']);
```

---

### ChangeLog

#### v4.2.26
* Bumped versions: `actix-web` 4.18.1, `log` 0.4.35, `redis` 1.8.1, `sentry`/`sentry-actix` 0.49.5, `tokio` 1.54.1, `utoipa-swagger-ui` 10.0.2, `smallvec` 1.16.4, and the rest of `Cargo.lock`. `parking_lot` 0.12 → 0.13 is the breaking one: it removes `read_recursive()`, which every call site replaced with `read()` (none of them takes the same lock twice), and the `hardware-lock-elision` feature, and needs Rust 1.95, so the minimum Rust version is now 1.95.0. The Docker image builds on `rust:1.99-alpine3.24`.
* **UDP accepts `udp://host:port/announce/<user key>` when `users_enabled` is on and `keys_enabled` is off**, matching HTTP's `/<user key>/announce`. UDP used to read the user key only from the third path segment, so the URL needed a 40-character placeholder for an announce key that was not used (`/announce/<placeholder>/<user key>`); that form keeps working. With keys on, the format is unchanged: `/announce/<key>/<user key>`.
* **Fixed user stats never being recorded over HTTP.** The HTTP announce checked the user key, then passed the key itself on as the user id. Users are stored under their id, so the lookup found nobody, and `uploaded`, `downloaded`, `completed` and `torrents_active` were never updated for HTTP announces. UDP was not affected.
* **Users' totals are written to the cache** (Redis or Memcache) under `<prefix>u:<id>`, next to the torrent keys, so a website can read user stats without the SQL database. `<id>` is the user's uuid or numeric id; the fields are `uploaded`, `downloaded`, `completed`, `active`, `updated` and `torrents`. User keys do not expire and are deleted with the user. With a cache, user changes are queued and flushed even without a persistent users table. See "User stats" under Caching. The torrent keys are unchanged.
* **`--import` writes users to the database only when the users table is persistent**, as it already did for torrents. Imported users also reach the cache.
* **The last peer's `stopped` now credits the user.** When that `stopped` emptied a non-persistent torrent and the torrent was dropped, the session's `uploaded` and `downloaded` were dropped with it.

#### v4.2.25
* **Fixed a whitelist with `persistent = false` not being loaded at startup (regression in v4.2.24).** v4.2.24 loaded a list only when its table was persistent, so a read-only whitelist, such as one pointed at the website's torrents table with `persistent = false`, came up empty. With `whitelist_enabled` on, every announce for a torrent not added over the API since the restart got `unknown info_hash`, and since those refusals are not counted as failures, the only sign was the seed and peer counts dropping. A list is loaded again whenever its mode is on, as before v4.2.24, and also when its table is persistent; `persistent = false` only stops it being written back.
* **Peer expiry no longer rewrites every idle torrent on every pass (regression in v4.2.24).** With `persistent` on, a torrent without announces for longer than the peer timeout stays expired, and each cleanup pass queued it again even when it had no peers left to remove. Every torrent loaded from the database was written to the database and to the cache every `peers_cleanup_interval`, and Redis kept a zero-count key for each one. A torrent is now queued only when the pass actually removed peers from it.

#### v4.2.24
* Bumped versions, including two breaking ones that needed no code changes: `utoipa-swagger-ui` 9 → 10 (and `utoipa` 5 → 6 with it) and `zstd` 0.13 → 0.14.
* Minimum Rust version is now 1.94.0. `sqlx` 0.9 already required it, so the declared 1.88.0 was wrong; `uuid` 1.28 and `actix-server` 2.10 need 1.89.
* Bumped `rtctorrent`'s npm dependencies within their major versions and rebuilt the shipped bundles; `npm audit` reports no vulnerabilities in the runtime dependencies. `bencode` stays on 2.x because 3.x and later are ESM-only and the library is CommonJS.
* Added `version.sh`, the Linux/macOS counterpart of `version.bat`. It updates the same files and leaves line endings and the final newline untouched.
* Added a `.gitattributes` (`* text=auto`) so a checkout on Windows and one on Linux no longer show every file as modified.
* The Linux build requirements in this README listed `fontconfig`, which nothing depends on anymore. They now list what the build actually needs: a C compiler and `curl`.
* **Fixed the tracker aborting at boot while loading users or torrents from the database**, and then failing the same way on every later start:
  * **SQLite:** a numeric-id user row aborted the load (the INTEGER id was decoded as bytes), and so did the first user past 4 GiB uploaded or downloaded (the counters were decoded as `u32`).
  * **MySQL:** `--create-database` makes the counters and the id `BIGINT UNSIGNED`, which the signed decoders refuse. They are now read whether a column is signed or `UNSIGNED`, so tables created elsewhere (such as the website's `torrust_actix_torrents`, whose `completed` is a signed `BIGINT`) keep loading.
  * **PgSQL:** the key `timeout` column is `integer` but was decoded as `bigint`.
  * **Numeric user ids** loaded from SQLite and MySQL are now hashed the way the API and PgSQL already hash them (`hash_id(id)`), so a user loaded from the database and the same user posted over the API share one entry.
* **Fixed announce keys added over the API getting the wrong expiry after a restart.** Memory held `now + timeout`, but the database was handed the bare `timeout` and read it back as an absolute timestamp, so a one-hour key was expired after any restart. A timeout of `0`, documented as permanent, expired immediately. Memory and the database now store the same absolute expiry, `0` stays permanent, and very large timeouts saturate instead of wrapping. `add_key` takes a `u64`. **Library API change.**
* **SIGTERM now shuts the tracker down cleanly.** The background tasks already stopped on it, but the main loop only waited for Ctrl-C, so `docker stop` and `systemctl stop` left the process hanging until it was killed, without the final save to the database. `docker/init.sh` now `exec`s the binary so it receives the signal.
* **Shutdown no longer rewrites `config.toml`.** It re-serialised the running configuration, which wrote secrets set through environment variables (`TRACKER__API_KEY`, the cluster token, the database DSN) into the file in plain text, dropped its comments and made temporary overrides permanent. Its stated purpose, carrying the completed count over without a database, never worked: it wrote the value from startup. `Configuration::save_from_config` is removed. **Library API change.**
* **Added `--migrate-database`, a one-off migration of the key `timeout` and user `updated` columns from 32-bit to 64-bit on MySQL and PostgreSQL** (see *Upgrading a MySQL or PostgreSQL database created before v4.2.24* above). Both columns hold Unix timestamps and would have overflowed on 19 January 2038; a key added with a long timeout could not be stored at all. `--create-database` now creates them as `BIGINT`, and the PostgreSQL loader reads either width, so the tracker keeps working before and after the migration.
* **Fixed every shutdown aborting before the final save to the database.** The UDP parse pool owns a tokio runtime, and the pool was released at the end of an async receive loop, where dropping a runtime panics. Under the release profile's `panic = 'abort'` the process died right there, after the listeners had stopped but before `Saving final data to database...`. The runtime is now disposed of with `shutdown_background()`.
* **Certificate hot reload now takes effect.** The HTTP and API TLS listeners served a key cached once at startup, so `POST /api/certificate/reload` reported success while the old certificate stayed in use until a restart. Handshakes now read the current certificate from the store, and the cluster master's TLS listener goes through the same store instead of loading its PEM files once at startup, so a reload renews it too (filter with `{"server_type": "websocket"}`). The signing key is built when the files are loaded, so a reload with an unusable key now fails the reload request instead of breaking later handshakes. `DynamicCertificateResolver::refresh_cache` is removed. **Library API change.**


##### Security (v4.2.24)
* **HTTP announces now honour `users_enabled`.** `/announce`, and `/{key}/announce` with keys enabled, carry no user key and were served anyway; they now get `unknown user key`, as UDP already answered.
* **A peer can no longer be stopped or moved by anyone who knows its peer id.** Peer ids are no secret (non-compact responses list them, and every BitTorrent handshake reveals one), so `event=stopped` with someone else's id emptied swarms, and a re-announce with it pointed the swarm at another address. A stored peer now only changes for a request from its own IP, or one carrying the same announce `key`, the parameter BitTorrent has for exactly this; clients that change address and send a key (libtorrent-based ones, and every UDP client) keep moving freely. A peer that has missed two announce intervals is no longer defended, so a client without a key that changed address gets through on its second announce from there; a live peer, which re-announces every interval, cannot be taken over.
* **An empty `api_key` no longer opens the API.** `?token=` matched it; an empty token is now always refused.
* **Removed HTTP/2** from the HTTP, API and cluster TLS listeners. actix offered it through ALPN, and the `h2` 0.3 underneath has RUSTSEC-2026-0258 (unbounded empty DATA frames) with no 0.3 fix. Everything a tracker serves is HTTP/1.1.
* **One oversized request no longer disconnects a cluster slave.** The slave forwarded any query string, and the master closed the whole cluster link on a frame over 64 KiB; slaves now apply the 8 KiB query cap before forwarding and the master accepts frames up to 1 MiB.
* **`rtctorrent = false` is now honoured in cluster mode**, where the slave used to forward WebRTC announces without checking it.
* A failure on the cluster master now reaches the client as a `failure reason` (HTTP) or no datagram (UDP), instead of an empty 200 or an empty datagram.
* UDP: a datagram too short to carry a transaction id gets no reply (it used to get 19 bytes back for 1 in, to an unverified source), malformed requests are answered with their own transaction id instead of 0, a refused request keeps its reason (`unknown key`, `torrent not on whitelist`, ...) instead of a generic `bad request`, and a UDP scrape is refused when `keys_enabled` is set, as an unkeyed HTTP scrape is.
* The generated `config.toml` is created owner-only (`0600`); it holds the API key, the cluster token and the database DSN.
* Database and cache connection strings are logged with the password masked.
* `cache.address` now accepts credentials, a host name and a database (`user:pass@host:port`, or a URL such as `redis://:pass@redis:6379/0`), so the Redis the website reads no longer has to run without a password. The key format is unchanged.
* Moved off `actix-web-actors`, which is deprecated upstream, to `actix-ws` for the cluster master's connections.
* Dropped OpenSSL: Sentry's `transport` feature pulled in `native-tls` beside the rustls everything else uses. Sentry now sends over reqwest with rustls, and OpenSSL is no longer a build requirement.
* Added `.cargo/audit.toml`, which ignores four advisories that were checked and found not to apply (`rsa`'s Marvin attack: sqlx only encrypts with the server's public key; three in `zip`, a build-only dependency whose affected functions are never called). The reasons sit next to each entry.

##### Fixes (v4.2.24)
* **The cache (and the website's peer counts) now follows peers leaving.** Peers that expired, and a `stopped` that emptied a torrent, never reached the update queue, so Redis kept showing them until its TTL (24 hours in a typical setup). Both now queue the new counts, or remove the key.
* **A big swarm no longer hands every client the same peers.** The peer list always started at the front of the swarm's map, so in a large swarm everyone got the same 72 peers. It now starts at a uniformly random peer and wraps around. The peer maps are `IndexMap`s instead of `HashMap`s for this: access by position makes the random start O(1), so announce cost stays bounded by `numwant` rather than by swarm size. Measured on `announce_into_populated_swarm`: about 1.7 µs per announce at both 200 and 1000 peers on the test machine, which includes the new peer-identity check.
* **The RtcTorrent signalling response honours `numwant`** (1..=72) like a BitTorrent peer list. It used to send every offer it had, up to 128 of up to 8 KiB each, while the client dials at most 8 per round.
* **Queued key, user, whitelist and blacklist updates can no longer overwrite each other.** They were keyed by the wall-clock nanosecond, which two updates can share (Windows ticks every 100 ns) and which can step backwards; they now use the same sequence counter as torrent updates.
* **The user update queue holds one entry per user**, instead of one full copy of the user per announce until the next flush.
* `torrents_active` entries for torrents a client left without `event=stopped` now expire after the peer timeout, instead of staying forever.
* Bulk `POST`/`DELETE /api/torrents` and `POST /api/users` validate the whole batch first: a bad entry used to return 400 after part of the batch (in map order) had already been applied.
* A UUID is now the same user however it is cased; it was hashed as sent but stored lowercased, so an uppercase UUID became a different user after a restart.
* Non-compact announce responses send `peer id` as the raw 20 bytes BEP 3 specifies (it went out as 40 hex characters) and leave it out for `no_peer_id`.
* io_uring: kernels without `IORING_FEAT_EXT_ARG` (before 5.11) now fall back to recvmmsg instead of every receive thread exiting at its first wait, and the receive buffers are no longer freed while the kernel may still be writing into them.
* `max_connections` under `[[http_server]]` and `[[api_server]]` is now applied; it was read, documented and ignored.
* **Whitelist, blacklist, key and user changes made over the API are saved even while that mode is off.** The API accepted them and queued them for the database, but the queue was only written, and the table only loaded at startup, while `whitelist_enabled` (or `blacklist_enabled`, `keys_enabled`, `users_enabled`) was on. With the mode off the queue grew without limit and the changes were lost at the next restart. Each table is now loaded and saved whenever it is persistent (`[database] persistent`, or the table's own `persistent`); the `*_enabled` switches only decide whether announces are checked against it, as before. A list can be filled first and its mode switched on later, and `DELETE /api/{whitelist,blacklist,keys,users}/clear` no longer answers `400 ... not enabled` while the mode is off. A whitelist that fails to load still stops the tracker while whitelist mode is on, and is logged otherwise.
* **Warnings about invalid environment overrides are shown.** They were logged before the logger was set up, so they never appeared; they now go to stderr, with the rest of the configuration checks.
* `cargo clippy --all-targets` is clean. The flush functions (`save_torrents`, `save_keys`, `save_users`, ... and their `_updates` variants) return the `sqlx::Error` they hit instead of `()`, and log it; the remote-IP lookups return `Option`; request helpers box their ready-made error response. **Library API change.** The UDP scrape parser loses an error branch that could not be reached.

##### Simplification (v4.2.24)
About 3,000 lines of Rust (14%) and four direct dependencies (`uuid`, `sha1`, `async-trait`, `tokio-shutdown`) are gone, with the API status codes, the Redis keys and the cluster wire format unchanged. What you can notice:
* **An unparsable number or boolean in an environment override now keeps the configured value** (with a warning on stderr) instead of resetting it to a built-in default, as `TRACKER__MAX_TORRENTS` already did. `DATABASE_STRUCTURE__*__PERSISTENT` still falls back to unset.
* Pending whitelist, blacklist and key updates hold one entry per hash, as user updates already did, so adding and removing the same hash before a flush writes only the last action. The `whitelist_updates`, `blacklist_updates` and `keys_updates` statistics count pending hashes, not API calls.
* The 400 bodies for a malformed hash are consistent across the API: `bad <field>` for a wrong length and `invalid <field>` for a non-hex character. `GET`/`POST`/`DELETE /api/torrent/{info_hash}` used to answer `bad info_hash` for both, and `DELETE /api/key/{key_hash}` said `invalid key`.
* `receive_method = "auto"` is now an alias of `recvmmsg` (which it always behaved as) rather than a separate value.
* A flush logs one `[SYNC ... UPDATES]` line instead of two, and a cluster master that cannot bind its address logs the reason and exits instead of panicking.

Removed or changed in the library API (**Library API change**):
* Dead code: the UDP response parsers (`Response::from_bytes`, `parse_ipv4_peers`, `parse_ipv6_peers`, `parse_scrape_stats`) and `Request::write`; the cluster `RequestType::ApiCall`/`ProtocolType::Api`, `process_api_call` and the per-encoding encode/decode helpers (`process_cluster_request` no longer takes an encoding); `src/ssl/ssl.rs`, `reload_all`, `reload_certificate_with_paths`; `hex2bin` (use `hex_to_id` or `str::parse::<InfoHash>()`), `validate_info_hash_hex`, `validate_peer_id_hex`, `convert_int_to_bytes`/`convert_bytes_to_int`, and about twenty unused tracker methods and constants.
* The `DatabaseBackend` and `CacheBackend` traits, each with one implementation, are gone; the connectors keep the same methods as inherent ones.
* The `TorrentTracker::save_torrents`/`save_keys`/`save_users`/`save_whitelist`/`save_blacklist` wrappers are removed; the `_updates` flushes call `DatabaseConnector` directly, and `TorrentTracker::save_all_updates` runs all of them (the periodic flush and the final save on shutdown use it). `get_key_updates` and `remove_key_update` are removed.
* `udp_service` and `UdpServer::new` take the `UdpTrackersConfig` instead of twelve arguments. `trusted_proxy_ips` and `proxy_addresses` are parsed straight into `Vec<IpAddr>`, so the shadow `trusted_proxy_addrs`/`proxy_addrs` fields are gone.
* `api_parse_body` became `api_json_body::<T>()` and `parse_info_hash` became `parse_hex_id::<T>()`; `DynamicCertificateResolver::server_config` builds a listener's rustls config.
* `InfoHash`, `PeerId` and `UserId` implement `Deserialize` from their hex form, and `CustomError` derives `thiserror::Error`.

##### Docker and CI (v4.2.24)
* **The image is built from the checkout it sits in** (`docker build -f docker/Dockerfile .` from the repository root) instead of cloning a git tag from GitHub, in two stages on pinned `rust:1.98-alpine3.24` / `alpine:3.24`: 108 MB instead of an image carrying the Rust toolchain and the whole build tree. A `.dockerignore` keeps `target/`, `config.toml` and databases out of the build context.
* The container runs as an unprivileged `tracker` user, and `init.sh` `exec`s the tracker so `docker stop` reaches it.
* **The healthcheck works.** The image copied `healthcheck.py` under the name of its PyInstaller build but had no Python, so the container was always unhealthy; the PyInstaller build itself was a one-directory build missing its `_internal/` folder, so it could not run either. The image now installs `python3` and runs the script, and the broken binary is removed.
* `docker-compose.yml` takes the API key from the environment (`TRACKER__API_KEY`, required) instead of a hard-coded one, and publishes the API on `127.0.0.1` only.
* GitHub Actions: workflows are read-only by default with write access only for the release job, checkouts no longer leave the token on disk for build scripts to find, third-party actions are pinned to commit SHAs and moved to their current majors (checkout v7, upload-artifact v7, download-artifact v8, which now fails on a hash mismatch, action-gh-release v3; all Node 24, none with an input change this workflow uses), the shell steps pass actionlint and shellcheck, and UPX is pinned to 5.2.1 and checked against its SHA-256. Dependabot targeted a `develop` branch that does not exist, so it never opened a PR; it now targets the default branch.
* `version.bat`/`version.sh` no longer touch the Dockerfile, which has no version in it any more.

##### RtcTorrent (v4.2.24)
* **Pieces from peers are now verified or rejected, never accepted unchecked.** A piece was accepted when the torrent had no v1 hashes (every pure v2 torrent), when its index was out of range, or when hashing failed (always, in the browser bundle outside a secure context). Pure v2 torrents are now checked against their files' `piece layers`; out-of-range or wrongly sized pieces are dropped; and without a hash function pieces are rejected. **Behaviour change:** a page serving the browser bundle over plain `http://` (other than `localhost`) can no longer download, since it has no `crypto.subtle`; serve it over HTTPS.
* A peer no longer decides how much memory a client allocates: piece chunks are only buffered for pieces this client requested, at exactly the piece's size.
* `seed.js --torrent-file` refuses paths that leave the working directory (`../`, absolute paths), which a crafted torrent could use to have the seeder read and serve any file.
* Failed and closed WebRTC connections are dropped, so a seeder that went away can be dialled again, and at most 8 new connections are opened per announce round.
* A stopped torrent clears its timers, so a Node.js process exits once its last torrent stops.
* Seeding from memory joins the files once instead of once per piece served.
* The bundles build without webpack's two "Critical dependency" warnings: the Node.js WebRTC packages are now loaded with plain `require()` calls (externals in the Node bundle, stubbed out in the browser one) instead of passing `require` around as a value.
* The demo server (`npm run serve`) streams files with HTTP range support (video seeking, and webseed fetches past the first piece, failed against it) and listens on `127.0.0.1` only.
* **Multi-file pure v2 torrents use the v2 layout.** BEP 52 starts every file on a piece boundary, and its per-file hashes cover exactly that; this library laid the files out back to back as in v1, so their pieces matched no hash. Seeding (from disk, memory or browser `File`s) and downloading now both use the piece-aligned layout. **Behaviour change:** for multi-file pure v2 torrents only, peers on this version and older ones no longer understand each other's piece numbers.
* **v2 torrents created by this library now follow BEP 52.** A file's final 16 KiB block was zero-padded before hashing, which libtorrent and other clients do not do, so they disagreed with this library's `pieces root` and info-hash for most files. Torrents created before this release still verify. `create()` also returns the info dictionary it actually encoded, so seeding the object it returns lays the data out as the torrent describes.
* **Hybrid (v1 + v2) torrents created by this library are now valid for other clients.** Their v1 file list was in the caller's order with the files back to back, while the v2 file tree is sorted and starts each file on a piece boundary, so libtorrent refused multi-file hybrids outright ("the v1 and v2 file metadata does not match") and found single-file ones inconsistent. The v1 list now follows the file tree with BEP 47 pad files, and the v1 piece hashes cover that layout. Checked with libtorrent 2.1.1: hybrid and v2, single- and multi-file torrents from this release load and verify fully; the same torrents from v4.2.23 were rejected or failed verification. **Behaviour change:** multi-file hybrid torrents made by older versions keep their old layout and still download between peers of this library.
* A seeder drops piece requests beyond twice what a leecher keeps in flight per connection, and ones outside the torrent, instead of queueing them without limit.
* Stopping a seeder while its first WebRTC offer is still being gathered now closes that connection; it used to stay open and keep a Node.js process running.
* `npm test` now runs a piece-verification suite (v1, hybrid, single- and multi-file v2 and hybrid with pad files, BEP 52 roots, the request cap) instead of failing with no tests.

#### v4.2.23
* Bumped versions.
* Fixed `POST /api/torrent/{info_hash}/{completed}` (and the bulk variant) wiping the swarm of an already known info-hash: it handed `add_torrent` a freshly built, peer-less entry, which replaced the live peer maps with empty ones and flushed those zeroes on to the database and the cache.
* Security: bumped `rustls` to 0.23.45 for RUSTSEC-2026-0285 (TLS 1.3 handshake messages accepted across encryption level boundaries).
* Security: the global statistics counters now saturate instead of wrapping. `update_counter` used `fetch_add`/`fetch_sub`, so a client-supplied magnitude — `completed` off the API path, a user's `uploaded`/`downloaded` off an announce — could drive a counter negative; `-value` also overflowed on `i64::MIN`.
* `set_torrent_completed` returns the counts to queue rather than the stored entry, so registering a hot torrent no longer clones its whole swarm under the shard write lock.
* The Swagger page's reported API version no longer lags the release; `version.bat` now bumps it too.

##### Cleanup (v4.2.23)
* **Deleted the `rtctorrent_bridge` module** (9 files). It shelled out to the RtcTorrent node CLI to create, seed and download torrents, and was reachable only through `create_rtctorrent`, which had no callers — as did `seed_torrent` and `download_torrent`. **Library API change.**
* **Deleted `QueryBuilder`** (187 lines). Sixteen of its seventeen methods had no callers at all and the seventeenth was only reached by its own tests; the database layer builds SQL with `format!` + `AssertSqlSafe` directly. **Library API change.**
* **`DatabaseConnector` and `CacheConnector` are enums instead of one `Option` per engine plus an `Option<engine>` discriminant.** Each had exactly one constructor filling exactly one engine, so "engine set but not connected", "two engines at once" and "no engine" were unreachable states that every method still answered for — 36 and 18 dead error arms between them. `database_connector.rs` drops 554 lines to 292 and `cache_connector_cache_backend.rs` 154 to 68, with the invalid combinations now unrepresentable. **Library API change.**
* **Deleted 13 public methods with no callers anywhere, including tests** (214 lines): `batch_contains_peers`, `clean_user_active_torrents`, `get_peers_ref`, `get_multiple_torrents`, `add_torrent_updates`, `add_user_active_torrent`, `create_rtctorrent`, `iter_all_torrents`, `get_torrents`, `backend`, `get_shard_content`, `get_shard` and `has_certificate`. The `#[allow(dead_code)]` on `impl TorrentSharding` that had been hiding most of them is gone too. **Library API change.**
* Deleted `ClusterMode::is_master`/`is_slave`/`is_standalone` — every caller already used `==` or `match` — and the `Ip` marker trait, whose zero methods only bounded two structs that are never instantiated with anything but `Ipv4Addr`/`Ipv6Addr`.
* `bin20_to_hex` hand-rolled a hex lookup table and loop eight lines below `bin2hex`, which already used `hex::encode_to_slice` from the same crate. It uses it now too.

##### Performance (v4.2.23)
* **Announce is no longer O(swarm size).** `AnnounceEntry::from_entry` cloned all six peer maps — up to 768 whole `TorrentPeer` values — inside the shard write lock, while a response reads at most two of them and only wants each peer's address and id. The BitTorrent maps now carry a 56-byte `ResponsePeer` in a `Vec` instead of a 104-byte `TorrentPeer` in a hash map (no per-peer hashing, one exact-size allocation), the caller says which half it needs, and the BitTorrent per-map cap drops from 128 to 73 because `numwant` is clamped to 72. The RtcTorrent maps keep their own `SNAPSHOT_RTC_PEER_CAP` of 128: the signalling response never consults `numwant`, so the number of signalling partners a WebRTC peer can see is unchanged. Measured on the new `announce_into_populated_swarm` benchmark: **about 7x faster at a 1000-peer swarm** (7.96 µs → 1.10 µs) and 2.8x at 200. The more useful figure is the scaling: growing a swarm from 200 to 1000 peers used to take an announce from 3.02 µs to 7.96 µs and now takes it from 1.09 µs to 1.11 µs, because the work is finally bounded by `numwant` rather than by swarm size.
* **The UDP parse queue no longer reserves 312 MB per listener.** `parse_queue_size` is now configurable under `[[udp_server]]` (env `UDP_n_PARSE_QUEUE_SIZE`) and defaults to 65536 slots — about 20 MB, down from a preallocated 1,000,000 — freeing ~292 MB per UDP listener, doubled when IPv4 and IPv6 servers both run.
* Reaching a shard no longer costs an atomic refcount round-trip: `TorrentSharding::shard_for` borrows the shard instead of cloning its `Arc`, on the thirteen call sites that immediately lock and drop it.
* The cluster announce builders used the cloning `get_peers`, so a forwarded announce copied its peers a second time; they now read the snapshot directly. Their unused `tracker` parameter is gone.
* Removed `RtcData::connection_status` and `update_rtc_connection_status`. The field held one of three literals, was allocated per RtcTorrent peer, reallocated on every state change and cloned on every announce — and nothing ever read it. **Library API change.**
* Dropped the dead `is_rtc_request` branches in the HTTP announce response builder; RtcTorrent requests return earlier and never reached them.
* Fixed a bug in `set_torrent_completed`'s statistics delta: converting each side to `i64` separately meant two `completed` values both above `i64::MAX` saturated to the same number and reported no change at all, however far apart they were. The difference is now taken in `u64` and only its magnitude clamped before the sign is applied.
* `parse_query` moved off SipHash to the project's `AHashMap` and skips percent-decoding for values holding no escape: about 11% off an announce query parse (1.49 µs → 1.32 µs). What remains is per-key allocation, which would need a borrowed key type to remove. Safe despite the keys coming off the wire — see below.
* Tried and reverted: giving each statistics counter its own cache line showed **no measurable difference** on an 8-thread announce benchmark (p = 0.91), so the ~3.2 KB and the extra indirection buy nothing at this core count. An earlier run of that benchmark suggested it was 6.5% slower, but the benchmark itself was at fault — every task cycled the whole info-hash set, so shard-lock contention swamped the counter traffic it was supposed to isolate. Fixed to give each task a disjoint slice, which is what produced the honest result.
* `tls_connection_rate` now actually does something. It was plumbed through config, documented and defaulted, but never read by anything. It is the cap on concurrent TLS handshakes per worker thread — the expensive half of a TLS connection — and is now passed to `HttpServer::max_connection_rate` on every TLS listener (HTTP, API and cluster master). **Config change:** `actix-tls` stores this in a single process-global, so three separate keys could never each take effect; `tracker_config.cluster_tls_connection_rate` is now `tracker_config.tls_connection_rate` (env `TRACKER__TLS_CONNECTION_RATE`) and the unreachable `[[http_server]]`/`[[api_server]]` keys of the same name are removed. Existing config files keep parsing — unknown keys are ignored — and pick up the 256 default, which is what they effectively had all along.
* Investigated and found to be a non-issue: `AHashMap` is `BuildHasherDefault<AHasher>`, which ahash's documentation calls "fixed keys" — but with the `std`/`runtime-rng` features (both on by default) those keys come from `getrandom` on first use and live in a process-wide `OnceBox`. "Fixed" means identical across every map within one run, not hardcoded: three runs of the test binary hash the same 20-byte value to three different results. Info-hash and peer-id collisions therefore cannot be precomputed, and no hasher change is needed.

#### v4.2.22
* Bumped versions.
* Security: moved off the yanked `chacha20` 0.10.1 that `rand`, `sqlx` and `tungstenite` pulled in.
* Bumped `rtctorrent`'s npm dependencies and rebuilt the shipped bundles.

#### v4.2.21
* Bumped versions.
* Dropped the unmaintained `rustls-pemfile` (RUSTSEC-2025-0134) for the PEM support already in `rustls-pki-types`; TLS key files may now be PKCS#8, PKCS#1 or SEC1 everywhere, including the cluster master.
* Security: `/scrape` without a key is now refused when `keys_enabled` is set, matching `/announce`.
* Security: cluster WebSocket connections that never complete the handshake are dropped after 30 seconds instead of holding a connection slot forever.
* Fixed two lifetime bugs and an alignment bug in the Windows RIO receive backend.
* The UDP hot path no longer builds a Sentry transaction per datagram when tracing is off.
* `hex2bin` rejects over-long input instead of silently truncating it to the first 20 bytes, and no longer reports malformed client input to Sentry.
* The announce path no longer aborts the process when the system clock sits before the UNIX epoch; it now uses the shared `current_time()` helper.
* Added the official Discord server: https://discord.gg/rC7aEtpwMX

#### v4.2.20
* Applied full audit and security scan on code, and applied recommended solutions.
* Optimization of over-engineered code, applied recommended alternatives.

#### v4.2.19
* Bumped versions.
* Fixed an exploit issue with fast-uri on the RtcTorrent library

#### v4.2.18
* Bugfix: A database connection loss or error, could freeze and lock up the update thread, should be fixed now.

#### v4.2.17
* Full code auditing, and found some possible security issues in the future.
* Applied a README fix about library installation on Linux (Thanks https://github.com/diederikdehaas).
* Some small version bumps, nothing special.

#### v4.2.16
* Added clearer error reporting for invalid hex-based identifiers.
* Tightened UUID and database identifier validation rules.
* Improved BEP 15 UDP request/response encoding and decoding for consistent big-endian behavior.
* Updated application/desktop/container version to 4.2.16 (including Docker image tags) and refreshed bundled dependency versions.
* Updated UDP parsing tests to match the revised packet encoding.

#### v4.2.15
* Did some further performance tweaking
* Bumped versions
* Added full doc blocks

#### v4.2.14
* Hotfix in regard to database handling
* A full audit and security scan of the code, shown some possible vulnerabilities
* Small little performance tweaks
* Adding a better token usage by recommending a header bearer, legacy still works

#### v4.2.13
* Applied a fix for Windows, implemented RIO for Windows
* Further optimization and some refactor work to improve performance and efficiency
* Minimum Rust version is 1.88.0

#### v4.2.12
* Applied UDP traffic control performance and improvements
* Introduced a new optional config for further optimization: receive_method. This can be "auto", "recvmmsg" or "io_uring". By default, "recvmmsg" is used, but io_uring can be used when you received a lot of traffic

#### v4.2.11
* Fixing a database (PostgreSQL) implementation code (Thanks zmiguel)
* Bumping version of SQLx, needed to modify the code to use the newer SQLx functions properly

#### v4.2.10
* Various audit findings and performance fixes

#### v4.2.9
* Bumping versions
* Locking versions, cause of chain attacks, making sure we use "safe" libs

#### v4.2.8
* Hotfix for the loading of data in memory, a SQL query issue

#### v4.2.7
* Added optional "persistent" override for each table
* Did another full audit scan and fixed possible issues

#### v4.2.6
* Hot Fix for the redis/memcache system, instead of having a persistent connection, only connect when needed
* Updated the readme for some information how to use the Cache system

#### v4.2.5
* Fixing the caching mechanism, it wasn't working as expected
* Added an option (it's optional, so it won't break anything) to either split up the seeds/peers list as 2 numbers or as separate numbers in BitTorrent IPv4, IPv6 and RtcTorrent.

#### v4.2.4
* Fixing a TLS/SSL bug where more than 1 provider was available, and server didn't know what to use

#### v4.2.3
* Changed some informative data
* Bug-fix needed for the peer/torrent cleanup thread

#### v4.2.2
* Fixing versioning, added back healthcheck for Docker and fixed some other small bugs
* Added `///` / `//!` doc comments throughout the codebase for docs.rs / crates.io publishing
* Made a large set of `[tracker_config]`, `[sentry_config]`, and `[database_structure.*]` keys optional in `config.toml` — sensible defaults are applied automatically when the keys are absent
  * Optional tracker fields: `whitelist_enabled`, `blacklist_enabled`, `keys_enabled`, `keys_cleanup_interval`, `users_enabled`, `swagger`, `prometheus_id`, all `cluster_*` fields, and all `rtc_*` fields
  * The entire `[sentry_config]` section can now be omitted (defaults to disable)
  * All `[database_structure.*]` subsections can be omitted (default table/column names are used)
* `--create-config` now annotates every optional key with an inline `# Optional: defaults to …` comment so new users can see at a glance what can be left out
* Refactored `impl RtcData` out of the struct file into `src/tracker/impls/rtc_data.rs`, consistent with the rest of the project structure

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
