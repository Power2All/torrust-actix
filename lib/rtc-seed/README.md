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
| `<FILE>...` | — | One or more files to seed (required) |
| `--tracker <URL>` | `http://127.0.0.1:6969/announce` | Tracker announce URL |
| `--name <NAME>` | First file's filename | Torrent display name |
| `--out <PATH>` | `<name>.torrent` | Output path for the `.torrent` file |
| `--webseed <URL>` | *(none)* | WebSeed URL (BEP-19); repeat for multiple |
| `--ice <URL>` | Google STUN servers | ICE server URL; repeat for multiple |
| `--rtc-interval <MS>` | `5000` | Tracker announce poll interval (milliseconds) |
| `--torrent-version` | `v1` | Torrent format: `v1`, `v2`, or `hybrid` (see below) |

### Multi-torrent mode (YAML)

```
rtc-seed --torrents <config.yaml>
```

Seed any number of torrents concurrently. Each torrent runs in its own async Tokio task.
When `--torrents` is used, all single-torrent flags are forbidden.

---

## Examples

### Seed a single file

```bash
./target/debug/rtc-seed video.mp4
```

### Seed to a remote tracker

```bash
./target/debug/rtc-seed \
  --tracker https://tracker.example.com/announce \
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

### Seed multiple torrents from a YAML file

```bash
./target/debug/rtc-seed --torrents /etc/rtc-seed/torrents.yaml
```

---

## YAML configuration file

Create a YAML file and pass it with `--torrents`. Each entry in the `torrents` list is seeded in parallel.

```yaml
---
torrents:
  # Minimal entry — only required fields
  - file:
      - "/data/movies/big_buck_bunny.mp4"
    trackers:
      - "http://tracker.example.com:6969/announce"

  # Full entry with all optional fields
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
```

### Field reference

| Field | Required | Description |
|---|---|---|
| `file` | **yes** | List of local file paths to seed |
| `trackers` | **yes** | List of tracker announce URLs (first URL is used) |
| `out` | no | Output path for the `.torrent` file |
| `name` | no | Torrent display name (default: first file's name) |
| `webseed` | no | WebSeed (BEP-19) HTTP URLs |
| `ice` | no | ICE server URLs (default: Google STUN servers) |
| `rtc_interval` | no | Announce poll interval in **milliseconds** (default: `5000`) |
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
7. **Prints upload stats** every 10 seconds: `[HH:MM:SS] peers: N  uploaded: X MB`.

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
