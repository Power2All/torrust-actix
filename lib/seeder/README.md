# seeder

A unified Rust seeder that serves torrent data over **BitTorrent** (BT wire protocol) and **WebRTC** (RTC data channels) simultaneously — or either one on its own.

`seeder` merges `bt-seed` and `rtc-seed` into a single binary. Both protocols share the same torrent file, piece data, tracker URL list, and upload counter. You choose the protocol globally or per-torrent via YAML or CLI.

---

## Table of contents

- [How it works](#how-it-works)
  - [BitTorrent mode](#bittorrent-mode)
  - [WebRTC mode](#webrtc-mode)
  - [Both mode](#both-mode)
  - [Shutdown](#shutdown)
- [Building](#building)
- [Usage — single-torrent (CLI)](#usage--single-torrent-cli)
- [Usage — multi-torrent (YAML)](#usage--multi-torrent-yaml)
  - [YAML format](#yaml-format)
  - [Global config keys](#global-config-keys)
  - [Torrent entry keys](#torrent-entry-keys)
- [Web management UI](#web-management-ui)
  - [Authentication](#authentication)
  - [Endpoints](#endpoints)
- [Protocol selection reference](#protocol-selection-reference)
- [Architecture overview](#architecture-overview)

---

## How it works

### BitTorrent mode

When `protocol` is `bt` or `both` (the default):

1. **Torrent creation** — the seeder hashes the piece data (SHA-1 v1, SHA-256 v2, or hybrid) and writes a `.torrent` file.
2. **Tracker announce** — announces `started` to each configured tracker URL in order, using HTTP (`http://`, `https://`) or the UDP tracker protocol (`udp://`). The first tracker that responds is used for subsequent re-announces.
3. **TCP listener** — binds a TCP port (default `6881`). In multi-torrent mode a single shared listener handles all torrents by looking up the info-hash from the BT handshake.
4. **Peer connection** — for each inbound TCP connection the seeder performs the BT handshake, sends a full bitfield, unchokes the peer, then fulfils `REQUEST` messages by reading blocks directly from disk (no full-file buffering).
5. **UPnP** — optionally maps the TCP port on the local gateway using `igd-next`.
6. **Re-announce** — a background task re-announces at the tracker-supplied interval.

### WebRTC mode

When `protocol` is `rtc` or `both`:

1. **Offer creation** — a `RTCPeerConnection` is created and an SDP offer is generated with ICE candidates gathered (up to 5 s timeout). The seeder always holds one unused offer ready to hand to the next incoming peer.
2. **Tracker announce** — announces to the first HTTP(S) tracker URL with the SDP offer encoded as `rtcoffer=`. The tracker stores the offer alongside the seeder's peer entry.
3. **Answer polling** — each signaling cycle the seeder re-announces and retrieves any pending `rtc_answers` from the tracker. For each answer it calls `set_remote_description` on the corresponding `PeerConnection`, completing the WebRTC handshake, and immediately creates a fresh offer for the next peer.
4. **Data channel** — once the WebRTC data channel opens, the seeder listens for `MSG_PIECE_REQUEST` frames (1-byte type `0x01` + 4-byte big-endian piece index). It reads the full piece from disk and replies with either a single `MSG_PIECE_DATA` frame (≤ 65 531 bytes) or multiple `MSG_PIECE_CHUNK` frames for large pieces.
5. **Re-announce interval** — controlled by `rtc_interval_ms` (default `5000` ms), overridden by the tracker's `rtc interval` response field.

### Both mode

When `protocol` is `both` (the default):

- All steps above run concurrently inside a single `tokio` runtime.
- Both protocols share the same `Arc<TorrentInfo>`, `Arc<AtomicU64>` upload counter, rate limiter, and peer ID.
- The BT TCP listener and RTC signaling loop are independent tasks coordinated by a `tokio::sync::watch` shutdown channel.
- Stats output shows `bt_peers`, `rtc_peers`, and `uploaded` together.

### Shutdown

On `Ctrl-C`:

1. The watch channel broadcasts `true` — all background tasks (BT listener, BT re-announce, RTC signaling) exit their loops.
2. The RTC task sends a `stopped` announcement before returning (up to 5 s timeout).
3. After tasks finish, any BT tracker also receives a `stopped` announcement (up to 5 s timeout).
4. In multi-torrent mode the shared TCP listener is aborted and the registry entry is removed.

---

## Building

```bash
# from the workspace root
cargo build -p seeder
cargo build -p seeder --release
```

The binary is placed at `target/debug/seeder` or `target/release/seeder`.

---

## Usage — single-torrent (CLI)

```
seeder [OPTIONS] [FILES]...
```

### Core flags

| Flag | Default | Description |
|---|---|---|
| `--protocol <PROTOCOL>` | `both` | `bt`, `rtc`, or `both` |
| `--tracker <URL>` | — | Tracker announce URL (repeatable). HTTP and UDP supported for BT; only HTTP for RTC |
| `--port <PORT>` | `6881` | BT TCP listen port |
| `--upnp` | false | Enable UPnP port mapping |
| `--ice <URL>` | Google STUN | ICE server URL (repeatable), e.g. `stun:stun.l.google.com:19302` |
| `--rtc-interval <MS>` | `5000` | WebRTC signaling poll interval in milliseconds |
| `--name <NAME>` | file name | Torrent display name |
| `--out <FILE>` | `<name>.torrent` | Path to write the `.torrent` file |
| `--torrent-version <VER>` | `v1` | `v1`, `v2`, or `hybrid` |
| `--torrent-file <FILE>` | — | Re-seed from an existing `.torrent` file |
| `--magnet <URI>` | — | Re-seed using a magnet URI |
| `--webseed <URL>` | — | Web-seed URL (repeatable) |
| `--upload-limit <KB/s>` | unlimited | Per-torrent upload rate cap |
| `--web-port <PORT>` | — | Start the web management UI on this port |
| `--web-password <PASS>` | — | Protect the web UI with a password |
| `--log-level <LEVEL>` | `info` | `error`, `warn`, `info`, `debug`, `trace` |

### Proxy flags

`--proxy-type`, `--proxy-host`, `--proxy-port`, `--proxy-user`, `--proxy-pass`
Supported types: `http`, `http_auth`, `socks4`, `socks5`, `socks5_auth`.

### Examples

```bash
# Seed a file over both BT and WebRTC
seeder --tracker http://tracker.example.com/announce movie.mkv

# BT only, custom port
seeder --protocol bt --port 51413 --tracker udp://tracker.opentrackr.org:1337/announce film.mkv

# WebRTC only, custom ICE, fast polling
seeder --protocol rtc \
  --tracker http://tracker.example.com/announce \
  --ice stun:stun.l.google.com:19302 \
  --rtc-interval 3000 \
  movie.mkv

# Re-seed an existing torrent over both protocols
seeder --torrent-file existing.torrent --tracker http://tracker.example.com/announce /data/movie.mkv

# Multi-file torrent
seeder --name "My Album" --tracker http://tracker.example.com/announce \
  track01.flac track02.flac track03.flac

# With web UI
seeder --protocol both --port 6881 \
  --tracker http://tracker.example.com/announce \
  --web-port 8092 --web-password secret \
  movie.mkv
```

---

## Usage — multi-torrent (YAML)

Pass a YAML config file with `--torrents`:

```bash
seeder --torrents torrents.yaml

# Override protocol and port from CLI
seeder --torrents torrents.yaml --protocol bt --port 6881

# With web UI (port from CLI overrides YAML)
seeder --torrents torrents.yaml --web-port 8092
```

If the YAML file does not exist, `seeder` creates an empty one and waits. The config is hot-reloaded when the file changes on disk, a `SIGHUP` is received (Unix), or the web UI triggers a reload.

### YAML format

```yaml
config:
  listen_port: 6881
  protocol: both                  # bt | rtc | both (default: both)
  rtc_ice_servers:
    - stun:stun.l.google.com:19302
    - stun:stun1.l.google.com:19302
  rtc_interval_ms: 5000
  upnp: false
  web_port: 8092
  web_password: secret
  # web_cert: /path/to/cert.pem
  # web_key:  /path/to/key.pem
  log_level: info
  show_stats: true
  proxy:
    proxy_type: socks5
    host: 127.0.0.1
    port: 1080
    # username: user
    # password: pass

torrents:
  - name: "My Movie"
    file:
      - /data/movie.mkv
    trackers:
      - http://tracker.example.com/announce
      - udp://tracker.opentrackr.org:1337/announce
    version: v1
    upload_limit: 10240           # KB/s; omit for unlimited
    enabled: true

  - name: "Music Album"
    file:
      - /data/album/track01.flac
      - /data/album/track02.flac
    trackers:
      - http://tracker.example.com/announce
    protocol: rtc                 # override global — RTC only for this torrent
    ice:
      - stun:custom.stun.example.com:3478
    rtc_interval: 3               # seconds (converted to ms internally)
    enabled: true

  - name: "Re-seed from .torrent"
    torrent_file: /data/existing.torrent
    file:
      - /data/existing_content/
    trackers: []                  # read from .torrent file automatically
    enabled: true
```

### Global config keys

| Key | Type | Default | Description |
|---|---|---|---|
| `listen_port` | `u16` | `6881` | BT TCP port shared by all torrents |
| `protocol` | `string` | `both` | Default protocol for all torrents |
| `rtc_ice_servers` | `[string]` | Google STUN x2 | Default ICE server list |
| `rtc_interval_ms` | `u64` | `5000` | Default RTC signaling interval (ms) |
| `upnp` | `bool` | `false` | Enable UPnP port mapping |
| `web_port` | `u16` | — | Web management UI port |
| `web_password` | `string` | — | Web UI password (bearer token auth) |
| `web_cert` | `path` | — | TLS certificate for HTTPS web UI |
| `web_key` | `path` | — | TLS private key for HTTPS web UI |
| `log_level` | `string` | `info` | Log verbosity |
| `show_stats` | `bool` | `true` | Print periodic peer/upload stats |
| `proxy` | `object` | — | Outbound proxy for tracker announces |

### Torrent entry keys

| Key | Type | Default | Description |
|---|---|---|---|
| `name` | `string` | file name | Torrent display name |
| `file` | `[path]` | — | Files or directories to seed (required unless `torrent_file` is set) |
| `trackers` | `[url]` | `[]` | Tracker announce URLs |
| `torrent_file` | `path` | — | Existing `.torrent` to re-seed |
| `magnet` | `string` | — | Magnet URI (tracker URLs extracted automatically) |
| `out` | `path` | `<name>.torrent` | Where to write the generated `.torrent` |
| `version` | `string` | `v1` | Torrent hash version: `v1`, `v2`, `hybrid` |
| `webseed` | `[url]` | — | Web-seed URLs embedded in the torrent |
| `upload_limit` | `u64` | — | Upload rate cap in KB/s |
| `protocol` | `string` | *(global)* | Per-torrent protocol override: `bt`, `rtc`, `both` |
| `ice` | `[url]` | *(global)* | Per-torrent ICE server list |
| `rtc_interval` | `u64` | *(global)* | Per-torrent RTC signaling interval in **seconds** |
| `enabled` | `bool` | `true` | Set `false` to skip this torrent without removing it |

> **Protocol resolution order:** per-torrent `protocol` → CLI `--protocol` → YAML `config.protocol` → `both`
> **ICE resolution order:** per-torrent `ice` → YAML `config.rtc_ice_servers` → Google STUN x2

---

## Web management UI

Start the web UI by setting `--web-port` or `config.web_port` in YAML. The UI is served at `http://host:<port>/`.

### Authentication

When a `web_password` is configured:

1. On first visit (or after session expiry) a **login modal** is shown — no page navigation, no HTTP Basic Auth prompt.
2. Enter the password; a `POST /api/login` request returns a **bearer token**.
3. The token is stored in `localStorage` as `seeder_token` and sent as `Authorization: Bearer <token>` on every subsequent API request.
4. Sessions expire after **1 hour** of inactivity. Each successful API call resets the timer.
5. The **Logout** button calls `POST /api/logout`, invalidates the server-side session, and returns to the login modal.

When no password is configured the UI is accessible without authentication.

### Password hashing (Argon2ID)

Passwords are stored and verified using **Argon2ID** — they are never stored in plain text. Use the built-in `hash-password` subcommand to generate a hash:

```bash
# Interactive (hidden input, confirmation prompt)
seeder hash-password

# Non-interactive (pipe-friendly)
seeder hash-password mysecretpassword
```

The command prints a PHC-format string such as:

```
$argon2id$v=19$m=19456,t=2,p=1$<salt>$<hash>
```

Store this string as the `web_password` value in your YAML config or pass it directly to `--web-password`:

```yaml
config:
  web_port: 8092
  web_password: "$argon2id$v=19$m=19456,t=2,p=1$<salt>$<hash>"
```

> **Note:** plain-text passwords are still accepted as a fallback for development convenience (any value that does not start with `$argon2` is compared literally). For production use always store a hashed value.

### Endpoints

| Method | Path | Description |
|---|---|---|
| `GET` | `/` | Web management UI (HTML) |
| `POST` | `/api/login` | `{"password":"…"}` → `{"token":"…"}` |
| `POST` | `/api/logout` | Invalidates the current bearer token |
| `GET` | `/api/status` | Live stats: uploaded bytes and peer count per torrent |
| `GET` | `/api/config` | Read global config |
| `PUT` | `/api/config` | Update global config (triggers hot-reload) |
| `GET` | `/api/torrents` | List all torrent entries |
| `POST` | `/api/torrents` | Add a torrent entry |
| `PUT` | `/api/torrents/{idx}` | Replace torrent entry at index |
| `DELETE` | `/api/torrents/{idx}` | Remove torrent entry at index |
| `GET` | `/api/browse?path=…` | Server-side file browser |

---

## Protocol selection reference

| Scenario | `protocol` value | BT listener | RTC signaling |
|---|---|---|---|
| Classic BitTorrent only | `bt` | Yes | No |
| WebRTC only (browser-compatible) | `rtc` | No | Yes |
| Serve both clients simultaneously | `both` | Yes | Yes |

A torrent entry with `protocol: bt` in a `both`-mode YAML session still benefits from the shared BT listener — it just won't make RTC offers. Similarly, a `protocol: rtc` entry skips the BT registry entirely.

---

## Architecture overview

```
seeder binary
│
├── config/
│   ├── enums/seed_protocol.rs     SeedProtocol { Bt, Rtc, Both }
│   └── structs/
│       ├── global_config.rs       YAML config: section (all fields)
│       ├── seeder_config.rs       Per-torrent runtime config
│       └── torrent_entry.rs       YAML torrents: entry
│
├── torrent/                       .torrent build + parse (v1/v2/hybrid)
│
├── tracker/
│   ├── structs/bt_client.rs       BtTrackerClient { Http | Udp }
│   └── structs/rtc_client.rs      RtcTrackerClient (HTTP-only + SDP offer)
│
├── seeder/
│   ├── seeder.rs                  BT wire handlers + RTC data channel handlers
│   ├── structs/seeder.rs          Seeder { peer_count (BT) + peers (RTC) + … }
│   ├── structs/torrent_registry.rs  Shared BT listener registry
│   ├── structs/peer_conn.rs       WebRTC PeerConnection wrapper
│   └── impls/seeder.rs            run() — concurrent BT+RTC with watch-channel shutdown
│
└── web/
    ├── api.rs                     REST API + bearer token auth
    ├── server.rs                  Actix-web server + optional TLS
    └── index.html                 UI: login modal, Protocol column, ICE/RTC settings
```

**Concurrency model inside `run()`:**

```
run()
 ├─ tokio::spawn  stats task (every 10 s)
 ├─ tokio::spawn  BT re-announce task   ──┐
 ├─ tokio::spawn  BT TCP accept loop    ──┤─ stopped via watch::channel(true)
 ├─ tokio::spawn  RTC signaling loop    ──┘
 └─ ctrl_c().await
       └─ stop_tx.send(true)
             ├─ BT stopped announce
             └─ RTC stopped announce (inside RTC task)
```
