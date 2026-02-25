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

```
rtc-seed [OPTIONS] <FILE>...
```

### Arguments

| Argument | Description |
|---|---|
| `<FILE>...` | One or more files to seed (required) |

### Options

| Flag | Default | Description |
|---|---|---|
| `--tracker <URL>` | `http://127.0.0.1:6969/announce` | Tracker announce URL |
| `--name <NAME>` | First file's filename | Torrent display name |
| `--out <PATH>` | `<name>.torrent` | Output path for the `.torrent` file |
| `--webseed <URL>` | *(none)* | WebSeed URL (BEP-19); repeat for multiple |
| `--ice <URL>` | Google STUN servers | ICE server URL; repeat for multiple |
| `--rtc-interval <MS>` | `5000` | Tracker announce poll interval (milliseconds) |

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
