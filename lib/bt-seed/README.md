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

### Seed multiple torrents from a YAML file

```bash
./target/debug/bt-seed --torrents /etc/bt-seed/torrents.yaml
```

---

## YAML configuration file

Create a YAML file and pass it with `--torrents`. Each entry in the `torrents` list is seeded in parallel.

```yaml
---
torrents:
  # Minimal entry — seed without announcing
  - file:
      - "/data/movies/big_buck_bunny.mp4"

  # With a single tracker
  - file:
      - "/data/movies/big_buck_bunny.mp4"
    trackers:
      - "http://tracker.example.com:6969/announce"

  # BEP-12 multi-tracker (all listed in announce-list)
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

### Field reference

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

---

## Reloading the YAML config

The Rust seeder supports **true in-process reload** — the process stays alive and all
seeder tasks are restarted without any process exit or process manager involvement.

This is possible because each torrent runs as an independent `tokio::spawn` task.
On reload, every task handle is `.abort()`ed (Tokio cancels the future at its next
`.await` point), the YAML file is re-read, and fresh tasks are spawned immediately.

### Reload triggers

| Platform | Trigger | How |
|---|---|---|
| **Linux / macOS** | `kill -HUP <pid>` | SIGHUP via `tokio::signal::unix` |
| **Windows + all** | Save the YAML file | File mtime polled every 2 s |
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
8. **Re-announces periodically** — uses the `interval` returned by the tracker; falls back to 30 minutes.
9. **Prints upload stats** every 10 seconds: `[HH:MM:SS] peers: N  uploaded: X MB`.

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
