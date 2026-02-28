# rtc-seed

A native Rust CLI seeder for [torrust-actix](../../README.md). It creates a `.torrent` file from one or more local files, prints a magnet URI, and seeds the content over WebRTC data channels — no Node.js required.

---

## Prerequisites

- **Rust** ≥ 1.75 (2021 edition)
- A running **torrust-actix** tracker with `rtctorrent = true` in the `[[http_server]]` config block

---

## Build

From the repository root:

```bash
cargo build -p rtc-seed
# Release build (recommended for seeding large files):
cargo build -p rtc-seed --release
```

The binary is written to `target/debug/rtc-seed` (or `target/release/rtc-seed`).

---

## Usage

### Single-torrent mode

```
rtc-seed [OPTIONS] <FILE>...
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
| `--ice <URL>` | Google STUN servers | ICE server URL; repeat for multiple |
| `--rtc-interval <MS>` | `5000` | Tracker announce poll interval (milliseconds) |
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

### Multi-torrent mode (YAML)

```
rtc-seed --torrents <config.yaml>
```

Seed any number of torrents concurrently. Each torrent runs in its own async Tokio task.
When `--torrents` is used, all single-torrent flags are forbidden.

---

## Examples

### Seed a single file (no tracker)

```bash
./target/debug/rtc-seed video.mp4
```

### Seed to a tracker

```bash
./target/debug/rtc-seed \
  --tracker https://tracker.example.com/announce \
  video.mp4
```

### Seed to multiple trackers (BEP-12)

```bash
./target/debug/rtc-seed \
  --tracker http://tracker1.example.com/announce \
  --tracker http://tracker2.example.com/announce \
  video.mp4
```

### Re-seed from an existing .torrent file (no re-hashing)

```bash
./target/debug/rtc-seed --torrent-file video.torrent
# Tracker URLs are read from the .torrent file automatically.
```

### Re-seed from a .torrent file with an explicit file path

```bash
./target/debug/rtc-seed --torrent-file video.torrent /data/movies/video.mp4
```

### Seed using a magnet URI (tracker from magnet)

```bash
./target/debug/rtc-seed \
  --magnet "magnet:?xt=urn:btih:...&tr=http%3A%2F%2Ftracker.example.com%2Fannounce" \
  video.mp4
```

### Seed multiple files with a custom name

```bash
./target/debug/rtc-seed \
  --name "My Movie Collection" \
  movie1.mp4 movie2.mp4
```

### Save the .torrent to a specific path

```bash
./target/debug/rtc-seed \
  --out /tmp/movie.torrent \
  video.mp4
```

### Add a WebSeed (HTTP fallback, BEP-19)

```bash
./target/debug/rtc-seed \
  --webseed https://cdn.example.com/files/video.mp4 \
  video.mp4
```

### Use a custom STUN/TURN server

```bash
./target/debug/rtc-seed \
  --ice stun:stun.example.com:3478 \
  video.mp4
```

### Seed with the web UI enabled

```bash
./target/debug/rtc-seed \
  --torrents torrents.yaml \
  --web-port 8091
# Open http://localhost:8091 in a browser to manage torrents live.
```

### Seed with the web UI + password + HTTPS

```bash
./target/debug/rtc-seed \
  --torrents torrents.yaml \
  --web-port 8091 \
  --web-password secret \
  --web-cert cert.pem \
  --web-key key.pem
```

### Seed with a SOCKS5 proxy for tracker announces

```bash
./target/debug/rtc-seed \
  --tracker http://tracker.example.com/announce \
  --proxy-type socks5 \
  --proxy-host 127.0.0.1 \
  --proxy-port 1080 \
  video.mp4
```

### Seed multiple torrents from a YAML file

```bash
./target/debug/rtc-seed --torrents /etc/rtc-seed/torrents.yaml
```

---

## YAML configuration file

Create a YAML file and pass it with `--torrents`. Each entry in the `torrents` list is seeded in parallel.

```yaml
---
# Optional global config (all fields are optional)
config:
  web_port: 8091                        # enables the web management UI
  web_password: "secret"                # HTTP Basic Auth (omit for no auth)
  web_cert: "/etc/ssl/certs/cert.pem"   # enables HTTPS (omit for plain HTTP)
  web_key: "/etc/ssl/private/key.pem"
  proxy:
    proxy_type: socks5                  # http | http_auth | socks4 | socks5 | socks5_auth
    host: "127.0.0.1"
    port: 1080
    username: "user"                    # optional (needed for http_auth, socks5_auth)
    password: "pass"

torrents:
  # Minimal entry — seed without announcing to any tracker
  - file:
      - "/data/movies/big_buck_bunny.mp4"

  # BEP-12 multi-tracker with upload cap and torrent version
  - out: "/var/torrents/sunflower.torrent"
    name: "Sunflower 1080p"
    file:
      - "/data/movies/sunflower_1080p.mp4"
    trackers:
      - "http://tracker.example.com:6969/announce"
      - "https://tracker2.example.com/announce"
    webseed:
      - "https://cdn.example.com/movies/sunflower_1080p.mp4"
    ice:
      - "stun:stun.l.google.com:19302"
    rtc_interval: 10000
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

  # Seed using a magnet URI (trackers parsed from it)
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
| `ice` | no | ICE server URLs (default: Google STUN servers) |
| `rtc_interval` | no | Announce poll interval in **milliseconds** (default: `5000`) |
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
| `proxy.proxy_type` | `http` \| `http_auth` \| `socks4` \| `socks5` \| `socks5_auth` |
| `proxy.host` | Proxy hostname or IP |
| `proxy.port` | Proxy port |
| `proxy.username` | Proxy username (optional) |
| `proxy.password` | Proxy password (optional) |

CLI flags (`--proxy-*`, `--web-*`) override the corresponding `config:` values when both are present.

---

## Web Management Interface

When `--web-port` is set (or `config.web_port` in YAML), rtc-seed starts an embedded Actix-web server serving a browser-based management UI.

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

rtc-seed can route tracker HTTP announces through a proxy.
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

The proxy applies to **tracker HTTP announces only**. WebRTC ICE/DTLS connections use their own ICE/STUN negotiation and are not affected.

---

## Reloading the YAML config

rtc-seed supports **true in-process reload** — the process stays alive and all
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
[rtc-seed] Config file changed on disk — reloading…
[rtc-seed] Applying new config…

[rtc-seed] Starting 3 torrent(s)…
[movie1.mp4] Hashing pieces… done.
...
```

### Error handling

If the YAML contains a syntax error after reload, the old tasks are still aborted
(there is no rollback to the previous config). Fix the file and trigger another
reload to recover.

### Empty YAML auto-creation

If `--torrents` points to a file that does not exist, rtc-seed creates an empty
`torrents.yaml` automatically and starts the web UI (if `--web-port` is set).
You can then add torrents through the browser without touching the file manually.

You can also omit `--torrents` entirely. If `--web-port` is given but no files
are provided, rtc-seed automatically creates `torrents.yaml` in the current
directory and enters multi-torrent mode with an empty list:

```bash
rtc-seed --web-port 8091
# Open http://localhost:8091 and add torrents through the browser.
```

### Contrast with the Node.js seeder

The Node.js `seed.js` uses `process.exit(0)` on reload instead of in-process restart.
This is because the `RtcTorrent` client has no public `destroy()` API — there is no
reliable way to stop the internal announce loops without exiting the process.
A process manager (pm2, systemd, forever) is therefore required for automatic restart
when using the Node.js seeder. The Rust seeder has no such requirement.

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
./target/debug/rtc-seed video.mp4

# Generate a pure v2 torrent (BEP-52 Merkle, no SHA-1 pieces)
./target/debug/rtc-seed --torrent-version v2 video.mp4

# Generate a hybrid torrent (both SHA-1 and SHA-256, widest compatibility)
./target/debug/rtc-seed --torrent-version hybrid video.mp4
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

> **Note:** SHA-256 Merkle piece verification on the leecher side is not yet implemented in `rtctorrent.js` (pure v2 pieces are accepted without verification). This is documented as future work.

---

## What it does

1. **Hashes the files** — splits content into pieces (16 KB for files ≤ 8 MB, 32 KB for larger), SHA-1 hashes each piece.
2. **Creates a `.torrent` file** — standard BitTorrent metainfo format, written to disk.
3. **Prints the magnet URI** — copy this into the demo player or any compatible leecher.
4. **Announces to the tracker** — sends an SDP WebRTC offer in the announce query (`rtctorrent=1&rtcoffer=...`).
5. **Waits for leechers** — polls the tracker on each `--rtc-interval`; when an SDP answer arrives it completes the WebRTC handshake.
6. **Serves pieces on demand** — leechers request pieces over the data channel; the seeder reads from disk and sends them. No full-file buffering in RAM.
7. **Enforces upload cap** — if `upload_limit` is set, blocks the send loop with a token-bucket rate limiter before each piece.
8. **Prints upload stats** every 10 seconds: `[HH:MM:SS] peers: N  uploaded: X MB`.

---

## Example output

```
=== RtcTorrent Seeder (Rust native) ===
Tracker : http://127.0.0.1:6969/announce
Files   : video.mp4

Creating torrent (hashing pieces)… done.
Saved : video.mp4.torrent
Hash  : 3a4b5c6d7e8f...

Magnet URI:
magnet:?xt=urn:btih:3a4b5c6d7e8f...&dn=video.mp4&tr=http%3A%2F%2F...

Share the magnet URI or the .torrent file with leechers.

Creating WebRTC offer (gathering ICE candidates)… done.
Seeding… (Ctrl+C to stop)

[14:32:10] peers: 1  uploaded: 12.4 MB
[14:32:20] peers: 1  uploaded: 48.0 MB
```

---

## Connecting a leecher

Open `lib/rtctorrent/demo/index.html` in a browser, enter:
- **Tracker URL**: same `--tracker` URL used above
- **Magnet URI** or **.torrent file**: the output from the seeder

The browser will connect to the Rust seeder over WebRTC and begin playback/download.

---

## Localhost testing — WebRTC mDNS warning

Chrome, Edge, and Firefox replace local IP addresses with `.local` mDNS hostnames in WebRTC ICE candidates. This breaks peer-to-peer connections when both sides are on the same machine. Disable it during local testing:

### Chrome / Edge

**Option A — command line flag:**
```bash
# Chrome
google-chrome --disable-features=WebRtcHideLocalIpsWithMdns

# Edge
msedge --disable-features=WebRtcHideLocalIpsWithMdns
```

**Option B — flags page (no restart required):**
1. Open `chrome://flags/#enable-webrtc-hide-local-ips-with-mdns` (or `edge://flags/...`)
2. Set to **Disabled**
3. Click **Relaunch**

### Firefox

1. Open `about:config`
2. Search for `media.peerconnection.ice.obfuscate_host_candidates`
3. Set to **`false`**

---

## Protocol reference

| Item | Value |
|---|---|
| Data channel label | `torrent` |
| Data channel ordered | `false` |
| Data channel max retransmits | `3` |
| ICE gathering timeout | 5 000 ms |
| `MSG_PIECE_REQUEST` | `0x01` + 4-byte piece index (big-endian) |
| `MSG_PIECE_DATA` | `0x02` + 4-byte piece index + piece bytes |
| `MSG_PIECE_CHUNK` | `0x04` + 4-byte index + 4-byte total size + 4-byte offset + chunk bytes |
| Chunk size (SCTP limit) | 16 KB (max SCTP payload: 65 531 bytes) |
