# bt-seed

A native Rust CLI seeder for [torrust-actix](../../README.md). It creates a `.torrent` file from one or more local files, prints a magnet URI, and seeds the content over the standard **BitTorrent wire protocol** (BEP-3) — no WebRTC, no Node.js, no external runtime required.

Any standard BitTorrent client (qBittorrent, Transmission, Deluge, aria2, …) can leech from it.

---

## Prerequisites

- **Rust** ≥ 1.75 (2021 edition)
- A running BitTorrent tracker (HTTP or UDP — any standard tracker works, including torrust-actix)

---

## Build

From the repository root:

```bash
cargo build -p bt-seed
# Release build (recommended for seeding large files):
cargo build -p bt-seed --release
```

The binary is written to `target/debug/bt-seed` (or `target/release/bt-seed`).

---

## Usage

### Single-torrent mode

```
bt-seed [OPTIONS] <FILE>...
```

| Argument / Flag | Default | Description |
|---|---|---|
| `<FILE>...` | — | One or more files to seed. Required unless `--torrent-file` is given. |
| `--tracker <URL>` | *(none)* | Tracker announce URL; repeat for multiple (BEP-12). If omitted, trackers from `--torrent-file` or `--magnet` are used. |
| `--torrent-file <FILE>` | *(none)* | Seed an existing `.torrent` file — reads tracker URLs and info_hash from it. No re-hashing needed. |
| `--magnet <URI>` | *(none)* | Seed using a magnet URI — reads tracker URLs from it; files are still hashed from disk. |
| `--name <NAME>` | First file's filename | Torrent display name |
| `--out <PATH>` | `<name>.torrent` | Output path for the `.torrent` file |
| `--webseed <URL>` | *(none)* | WebSeed URL (BEP-19); repeat for multiple |
| `--port <PORT>` | `6881` | TCP port to listen for incoming peer connections |
| `--torrent-version` | `v1` | Torrent format: `v1`, `v2`, or `hybrid` (see below) |
| `--web-port <PORT>` | *(disabled)* | Enable the web management UI on this port |
| `--web-password <PASS>` | *(none)* | Protect the web UI with HTTP Basic Auth |
| `--web-cert <FILE>` | *(none)* | PEM certificate file — enables HTTPS for the web UI |
| `--web-key <FILE>` | *(none)* | PEM private key file — required when `--web-cert` is set |
| `--proxy-type <TYPE>` | *(none)* | Proxy type for tracker announces: `http`, `http_auth`, `socks4`, `socks5`, `socks5_auth` |
| `--proxy-host <HOST>` | *(none)* | Proxy hostname or IP |
| `--proxy-port <PORT>` | *(none)* | Proxy port |
| `--proxy-user <USER>` | *(none)* | Proxy username (required for `http_auth` / `socks5_auth`) |
| `--proxy-pass <PASS>` | *(none)* | Proxy password |
| `--upnp` | `false` | Attempt UPnP IGD port mapping at startup |

### Multi-torrent mode (YAML)

```
bt-seed --torrents <config.yaml>
```

Seed any number of torrents concurrently. Each torrent runs in its own async Tokio task.
When `--torrents` is used, all single-torrent flags are forbidden.

---

## Examples

### Seed a single file (no tracker)

```bash
./target/debug/bt-seed video.mp4
```

### Seed to a tracker

```bash
./target/debug/bt-seed \
  --tracker https://tracker.example.com/announce \
  video.mp4
```

### Seed to multiple trackers (BEP-12)

```bash
./target/debug/bt-seed \
  --tracker http://tracker1.example.com/announce \
  --tracker udp://tracker2.example.com:6969/announce \
  video.mp4
```

### Re-seed from an existing .torrent file (no re-hashing)

```bash
./target/debug/bt-seed --torrent-file video.torrent
# Tracker URLs are read from the .torrent file automatically.
# File path is inferred from the torrent's name (resolved from CWD).
```

### Re-seed from a .torrent file with an explicit file path

```bash
./target/debug/bt-seed --torrent-file video.torrent /data/movies/video.mp4
```

### Seed using a magnet URI (tracker from magnet)

```bash
./target/debug/bt-seed \
  --magnet "magnet:?xt=urn:btih:...&tr=http%3A%2F%2Ftracker.example.com%2Fannounce" \
  video.mp4
```

### Seed to a UDP tracker (BEP-15)

```bash
./target/debug/bt-seed \
  --tracker udp://tracker.example.com:6969/announce \
  video.mp4
```

### Seed multiple files with a custom name

```bash
./target/debug/bt-seed \
  --name "My Movie Collection" \
  movie1.mp4 movie2.mp4
```

### Save the .torrent to a specific path

```bash
./target/debug/bt-seed \
  --out /tmp/movie.torrent \
  video.mp4
```

### Add a WebSeed (HTTP fallback, BEP-19)

```bash
./target/debug/bt-seed \
  --webseed https://cdn.example.com/files/video.mp4 \
  video.mp4
```

### Listen on a custom port

```bash
./target/debug/bt-seed \
  --port 51413 \
  video.mp4
```

### Seed with the web UI enabled

```bash
./target/debug/bt-seed \
  --torrents torrents.yaml \
  --web-port 8090
# Open http://localhost:8090 in a browser to manage torrents live.
```

### Seed with the web UI + password + HTTPS

```bash
./target/debug/bt-seed \
  --torrents torrents.yaml \
  --web-port 8090 \
  --web-password secret \
  --web-cert cert.pem \
  --web-key key.pem
```

### Seed with a SOCKS5 proxy for tracker announces

```bash
./target/debug/bt-seed \
  --tracker http://tracker.example.com/announce \
  --proxy-type socks5 \
  --proxy-host 127.0.0.1 \
  --proxy-port 1080 \
  video.mp4
```

### Enable UPnP port mapping

```bash
./target/debug/bt-seed \
  --upnp \
  --port 6881 \
  video.mp4
```

### Seed multiple torrents from a YAML file

```bash
./target/debug/bt-seed --torrents /etc/bt-seed/torrents.yaml
```

---

## YAML configuration file

Create a YAML file and pass it with `--torrents`. Each entry in the `torrents` list is seeded in parallel.

```yaml
---
# Optional global config (all fields are optional)
config:
  web_port: 8090                        # enables the web management UI
  web_password: "secret"                # HTTP Basic Auth (omit for no auth)
  web_cert: "/etc/ssl/certs/cert.pem"   # enables HTTPS (omit for plain HTTP)
  web_key: "/etc/ssl/private/key.pem"
  upnp: true                            # attempt UPnP IGD port mapping
  proxy:
    proxy_type: socks5                  # http | http_auth | socks4 | socks5 | socks5_auth
    host: "127.0.0.1"
    port: 1080
    username: "user"                    # optional (needed for http_auth, socks5_auth)
    password: "pass"

torrents:
  # Minimal entry — seed without announcing
  - file:
      - "/data/movies/big_buck_bunny.mp4"

  # With a single tracker
  - file:
      - "/data/movies/big_buck_bunny.mp4"
    trackers:
      - "http://tracker.example.com:6969/announce"

  # BEP-12 multi-tracker with upload cap and torrent version
  - out: "/var/torrents/sunflower.torrent"
    name: "Sunflower 1080p"
    file:
      - "/data/movies/sunflower_1080p.mp4"
    trackers:
      - "udp://tracker.example.com:6969/announce"
      - "https://tracker2.example.com/announce"
    webseed:
      - "https://cdn.example.com/movies/sunflower_1080p.mp4"
    port: 51413
    version: hybrid
    upload_limit: 2048                  # cap upload at 2048 KB/s (2 MB/s)

  # Temporarily disable a torrent without removing it
  - file:
      - "/data/movies/old_movie.mp4"
    trackers:
      - "http://tracker.example.com:6969/announce"
    enabled: false

  # Re-seed from an existing .torrent (no re-hashing, trackers read from file)
  - torrent_file: "/var/torrents/movie.torrent"

  # Re-seed from an existing .torrent with an explicit file path
  - torrent_file: "/var/torrents/movie.torrent"
    file:
      - "/data/movies/movie.mp4"

  # Seed using a magnet URI (trackers parsed from it, files hashed from disk)
  - magnet: "magnet:?xt=urn:btih:...&tr=http%3A%2F%2Ftracker.example.com%2Fannounce"
    file:
      - "/data/movies/movie.mp4"
```

### Per-torrent field reference

| Field | Required | Description |
|---|---|---|
| `file` | **yes*** | List of local file paths to seed (*required unless `torrent_file` is set) |
| `trackers` | no | List of tracker announce URLs (all included in `announce-list`; BEP-12). If omitted, trackers are read from `torrent_file` or `magnet`. |
| `torrent_file` | no | Path to an existing `.torrent` file; tracker URLs and info_hash are read from it. |
| `magnet` | no | Magnet URI; tracker URLs are parsed from it. Files are still hashed from disk. |
| `out` | no | Output path for the `.torrent` file |
| `name` | no | Torrent display name (default: first file's name) |
| `webseed` | no | WebSeed (BEP-19) HTTP URLs |
| `port` | no | TCP listen port (default: `6881`) |
| `version` | no | Torrent format: `"v1"` (default), `"v2"`, or `"hybrid"` |
| `enabled` | no | Set to `false` to skip this torrent without removing it from the config (default: `true`) |
| `upload_limit` | no | Upload speed cap in **KB/s** for this torrent (omit for unlimited) |

### Global config field reference (`config:`)

| Field | Description |
|---|---|
| `web_port` | Port for the web management UI; omit to disable the UI |
| `web_password` | HTTP Basic Auth password; omit for no authentication |
| `web_cert` | PEM certificate path — enables HTTPS |
| `web_key` | PEM private key path — required when `web_cert` is set |
| `upnp` | `true` to attempt UPnP IGD port mapping at startup |
| `proxy.proxy_type` | `http` \| `http_auth` \| `socks4` \| `socks5` \| `socks5_auth` |
| `proxy.host` | Proxy hostname or IP |
| `proxy.port` | Proxy port |
| `proxy.username` | Proxy username (optional) |
| `proxy.password` | Proxy password (optional) |

CLI flags (`--proxy-*`, `--web-*`, `--upnp`) override the corresponding `config:` values when both are present.

---

## Web Management Interface

When `--web-port` is set (or `config.web_port` in YAML), bt-seed starts an embedded Actix-web server serving a browser-based management UI.

### Features

- **Live torrent table** — shows each torrent's name, enabled state, upload cap, bytes uploaded, and current peer count; auto-refreshes every 5 seconds
- **Add torrent** — fill in the form to append a new entry to the YAML and start seeding immediately (no restart required)
- **Edit torrent** — update any field inline; changes are written to YAML and the seeder reloads
- **Toggle enabled** — one-click enable/disable without removing the entry from the config
- **Delete torrent** — removes the entry from the YAML and stops seeding it

### REST API

| Method | Path | Description |
|---|---|---|
| `GET` | `/` | Web UI (HTML page) |
| `GET` | `/api/status` | JSON: per-torrent `uploaded` bytes and `peer_count` |
| `GET` | `/api/torrents` | JSON: full `TorrentsFile.torrents` list |
| `POST` | `/api/torrents` | Append a new torrent entry (JSON body) |
| `PUT` | `/api/torrents/{idx}` | Replace the entry at index `idx` (JSON body) |
| `DELETE` | `/api/torrents/{idx}` | Remove the entry at index `idx` |

All mutating endpoints write the updated YAML to disk and trigger an in-process reload.

### Authentication

Set `config.web_password` (or `--web-password`) to enable HTTP Basic Auth.
Username can be any non-empty string; only the password is checked.

### HTTPS

Provide `web_cert` and `web_key` (PEM format) to serve the UI over HTTPS.
If only one is provided, the UI falls back to plain HTTP and a warning is logged.

Generate a self-signed certificate for local testing:

```bash
openssl req -x509 -newkey rsa:2048 \
  -keyout key.pem -out cert.pem \
  -days 365 -nodes -subj "/CN=localhost"
```

---

## Upload Rate Limiting

Set `upload_limit` (KB/s) in a torrent entry to cap the upload speed for that torrent.
The limit is enforced per-connection using a token-bucket rate limiter (governor crate).

```yaml
torrents:
  - file: ["/data/movies/big_file.mp4"]
    trackers: ["http://tracker.example.com/announce"]
    upload_limit: 1024   # max 1 MB/s upload for this torrent
```

Omit `upload_limit` (or set it to `null`) for unlimited upload speed.

---

## Proxy Support

bt-seed can route tracker HTTP announces through a proxy.
Configure it in the `config:` section of the YAML or via `--proxy-*` CLI flags.

```yaml
config:
  proxy:
    proxy_type: socks5
    host: "127.0.0.1"
    port: 1080
```

Supported proxy types:

| `proxy_type` | Protocol |
|---|---|
| `http` | HTTP CONNECT proxy (no auth) |
| `http_auth` | HTTP CONNECT proxy with Basic Auth |
| `socks4` | SOCKS4 |
| `socks5` | SOCKS5 (no auth) |
| `socks5_auth` | SOCKS5 with username/password |

The proxy applies to **tracker HTTP announces only**. Inbound TCP peer connections are not affected (they come from leechers dialing in, not outbound connections from bt-seed).

---

## UPnP Port Mapping

Pass `--upnp` (or set `config.upnp: true` in YAML) to have bt-seed attempt to map its listen port on your router via UPnP IGD at startup.

```bash
./target/debug/bt-seed --upnp --port 6881 video.mp4
```

bt-seed will:
1. Discover the local IP by connecting a UDP socket to `8.8.8.8:80`
2. Search for a UPnP gateway on the LAN
3. Request a TCP port mapping for the listen port

If UPnP fails (no gateway, mapping denied) a warning is logged and seeding continues normally without port mapping.

---

## Reloading the YAML config

bt-seed supports **true in-process reload** — the process stays alive and all
seeder tasks are restarted without any process exit or process manager involvement.

This is possible because each torrent runs as an independent `tokio::spawn` task.
On reload, every task handle is `.abort()`ed (Tokio cancels the future at its next
`.await` point), the YAML file is re-read, and fresh tasks are spawned immediately.

### Reload triggers

| Platform | Trigger | How |
|---|---|---|
| **Linux / macOS** | `kill -HUP <pid>` | SIGHUP via `tokio::signal::unix` |
| **Windows + all** | Save the YAML file | File mtime polled every 2 s |
| **Any** | Web UI add / edit / delete / toggle | REST API writes YAML and signals reload |
| **Any** | `Ctrl+C` | Clean shutdown — all tasks aborted, process exits |

### Console output on reload

```
[bt-seed] Config file changed on disk — reloading…
[bt-seed] Applying new config…

[bt-seed] Starting 3 torrent(s)…
[movie1.mp4] Hashing pieces (v1)… done.
...
```

### Error handling

If the YAML contains a syntax error after reload, the old tasks are still aborted
(there is no rollback to the previous config). Fix the file and trigger another
reload to recover.

### Empty YAML auto-creation

If `--torrents` points to a file that does not exist, bt-seed creates an empty
`torrents.yaml` automatically and starts the web UI (if `--web-port` is set).
You can then add torrents through the browser without touching the file manually.

You can also omit `--torrents` entirely. If `--web-port` is given but no files
are provided, bt-seed automatically creates `torrents.yaml` in the current
directory and enters multi-torrent mode with an empty list:

```bash
bt-seed --web-port 8090
# Open http://localhost:8090 and add torrents through the browser.
```

---

## BEP-52 (BitTorrent v2 / Hybrid) Support

The `--torrent-version` flag selects the `.torrent` format to generate:

| Value | Hash algorithm | `info` dict keys | Magnet URI |
|---|---|---|---|
| `v1` *(default)* | SHA-1, 20 bytes | `name`, `piece length`, `pieces`, `files`/`length` | `xt=urn:btih:<40-hex>` |
| `v2` | SHA-256 Merkle (BEP-52) | `file tree`, `meta version`, `name`, `piece length` | `xt=urn:btmh:1220<64-hex>` |
| `hybrid` | Both SHA-1 and SHA-256 Merkle | all of the above combined | `xt=urn:btih:<40-hex>&xt=urn:btmh:1220<64-hex>` |

**Tracker announce info_hash:**
- `v1` / `hybrid` → SHA-1 of `info` dict (20 bytes) — backward compatible with all trackers
- `v2` only → first 20 bytes of SHA-256 of `info` dict

**BEP-52 Merkle tree details:**
- Block size: 16 KiB (fixed by spec)
- Leaf hashes: SHA-256 of each 16 KiB block; last block zero-padded to 16 KiB
- Leaf count padded to next power of two with 32-zero-byte padding leaves
- `piece layers`: top-level torrent key (not in `info`); only for files larger than one piece
- `pieces root`: omitted for empty files

**Examples:**

```bash
# Generate a standard v1 torrent (default)
./target/debug/bt-seed video.mp4

# Generate a pure v2 torrent (BEP-52 Merkle, no SHA-1 pieces)
./target/debug/bt-seed --torrent-version v2 video.mp4

# Generate a hybrid torrent (both SHA-1 and SHA-256, widest compatibility)
./target/debug/bt-seed --torrent-version hybrid video.mp4
```

YAML example with per-entry version:

```yaml
torrents:
  - file: ["/data/old_client.mp4"]
    trackers: ["http://tracker:6969/announce"]
    version: v1        # default — compatible with all clients

  - file: ["/data/new_content.mp4"]
    trackers: ["http://tracker:6969/announce"]
    version: hybrid    # v1 + v2 — best of both worlds

  - file: ["/data/v2_only.mp4"]
    trackers: ["http://tracker:6969/announce"]
    version: v2        # pure BEP-52, no SHA-1
```

---

## Tracker protocol support

| Scheme | Protocol | Spec |
|---|---|---|
| `http://` / `https://` | HTTP GET announce | BEP-3 |
| `udp://` | UDP tracker protocol | BEP-15 |

The correct client is selected automatically based on the URL scheme.

---

## What it does

1. **Hashes the files** — splits content into pieces, SHA-1 (and/or SHA-256 Merkle for v2/hybrid) hashes each piece.
2. **Creates a `.torrent` file** — standard BitTorrent metainfo format (v1/v2/hybrid), written to disk.
3. **Prints the magnet URI** — share this with any BitTorrent leecher.
4. **Announces to the tracker** — sends a standard BT `started` announce with `uploaded=0`, `downloaded=0`, `left=0`, `compact=1`.
5. **Listens for peers** — opens a TCP listener on `--port` (default `6881`); accepts inbound connections from leechers.
6. **Handshakes peers** — validates the BT handshake (protocol string + info_hash), sends bitfield (all pieces available) and unchoke.
7. **Serves pieces on demand** — responds to `REQUEST` messages with `PIECE` messages; reads blocks directly from disk — no full-file buffering in RAM.
8. **Enforces upload cap** — if `upload_limit` is set, blocks the send loop with a token-bucket rate limiter before each piece block.
9. **Re-announces periodically** — uses the `interval` returned by the tracker; falls back to 30 minutes.
10. **Prints upload stats** every 10 seconds: `[HH:MM:SS] peers: N  uploaded: X MB`.

---

## Example output

```
=== BtSeed (Rust native) ===
Tracker : http://127.0.0.1:6969/announce
Files   : video.mp4
Port    : 6881

Creating torrent (hashing pieces)… done.
Saved : video.mp4.torrent
Hash  : 3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f1a2b

Magnet URI:
magnet:?xt=urn:btih:3a4b5c6d7e8f9a0b1c2d...&dn=video.mp4&tr=http%3A%2F%2F...

Share the magnet URI or the .torrent file with leechers.

Listening on 0.0.0.0:6881
Seeding… (Ctrl+C to stop)

[14:32:10] peers: 1  uploaded: 12.4 MB
[14:32:20] peers: 1  uploaded: 48.0 MB
```

---

## Protocol reference

| Item | Value |
|---|---|
| Peer ID prefix | `-BS1000-` followed by 12 random digits |
| Handshake length | 68 bytes (`\x13BitTorrent protocol` + 8 reserved + 20-byte info_hash + 20-byte peer_id) |
| Max block size | 16 KiB |
| Bitfield | All pieces set to `1` (seeder has complete file) |
| `MSG_CHOKE` | `0` |
| `MSG_UNCHOKE` | `1` |
| `MSG_INTERESTED` | `2` |
| `MSG_NOT_INTERESTED` | `3` |
| `MSG_HAVE` | `4` |
| `MSG_BITFIELD` | `5` |
| `MSG_REQUEST` | `6` |
| `MSG_PIECE` | `7` |
| `MSG_CANCEL` | `8` |
