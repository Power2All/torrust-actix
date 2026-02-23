# RtcTorrent

A WebRTC-enabled BitTorrent client library. Seeders and leechers communicate over WebRTC data channels, using a standard HTTP BitTorrent tracker for signaling (SDP offer/answer exchange). Works in both Node.js and the browser.

## Prerequisites

- **torrust-actix tracker** running with `rtctorrent = true` in the `[[http_server]]` config block
- **Node.js** ≥ 12
- For Node.js WebRTC: `@roamhq/wrtc` native module (installed automatically via `npm install`)

---

## Setup

```bash
cd lib/rtctorrent
npm install
npm run build       # produces dist/rtctorrent.browser.js and dist/rtctorrent.node.js
```

---

## Node-to-Node Test

Two automated tests verify the full stack without a browser.

### 1. Signaling flow test (no file transfer)

Validates the tracker's offer/answer signaling without requiring WebRTC hardware:

```bash
node test/test_signaling_flow.js
# Expected: 13 passed, 0 failed
```

### 2. End-to-end transfer test

Creates a small test file, seeds it from one Node process and downloads it in another, all in a single script using `@roamhq/wrtc` for real WebRTC connections:

```bash
# Make sure the tracker is running first
node test/test_webrtc_transfer.js
```

Expected output:
```
=== WebRTC Transfer Test ===

Test file: /tmp/rtctest_<timestamp>.bin  (5600 bytes)

--- Creating torrent ---
Info hash: ...

--- Starting seeder ---
[Seeder] SDP offer created (... bytes)

--- Starting leecher ---
[Leecher] Data channel opened
[Leecher] Piece 0 OK (5600 B) — 1/1 pieces
[LEECHER] Download complete!

=== Verification ===
Data match: PASS ✓
```

Or use the npm shortcut:

```bash
npm run test:transfer
```

---

## Manual Node Seed / Leech

### Seed files from the command line

```bash
node bin/seed.js [--tracker <url>] [--name <name>] [--out <file.torrent>] <file1> [<file2> ...]
```

**Examples:**

```bash
# Seed a single file using the default tracker (http://127.0.0.1:6969/announce)
node bin/seed.js /path/to/movie.mp4

# Seed with a custom tracker and output name
node bin/seed.js --tracker http://mytracker:6969/announce --name "My Movie" /path/to/movie.mp4
```

The seeder will:
1. Hash the file and create a `.torrent` file
2. Print the magnet URI
3. Start seeding (pieces are read on-demand — no full file loaded into RAM)

Share the printed magnet URI or the `.torrent` file with leechers.

---

## Browser Demo

The demo page lets you seed and download torrents entirely in the browser.

### 1. Build the browser bundle

```bash
npm run build
```

### 2. Start the demo server

```bash
npm run serve
# or: node bin/serve.js [port]   (default port: 8080)
```

Then open **http://localhost:8080/demo/** in your browser.

> No Python required. The server is a dependency-free Node.js script that serves static files.

### 3. Seed tab — browser seeder

**Option A — Seed an existing torrent**
1. Click **Load .torrent file** and select a `.torrent` file
2. Click **Select Files / Folder** and pick the matching file(s)
   - The page validates each file against the torrent (name + size must match)
   - Files with size mismatches are rejected; name-only mismatches show a warning
3. Click **Start Seeding**

**Option B — Create a new torrent**
1. Click **Select Files / Folder** and pick your files
2. Enter the tracker URL (default: `http://127.0.0.1:6969/announce`)
3. Optionally set a torrent name
4. Click **Start Seeding** — the torrent is created and seeding begins immediately
5. Use the **Download .torrent** or **Copy Magnet** buttons to share with leechers

### 4. Download tab — browser leecher

1. Enter a magnet URI **or** click **Load .torrent file** to load a `.torrent`
2. Enter the tracker URL if not already filled
3. Click **Download**
4. For MP4 files: playback starts as soon as the first contiguous pieces are received (progressive streaming)
5. Use **Save File** once the download completes, or let the video play directly

---

## Seeder in Node, Leecher in Browser (or vice versa)

This is the typical production workflow:

1. **Seed from Node:**
   ```bash
   node bin/seed.js --tracker http://127.0.0.1:6969/announce /path/to/movie.mp4
   ```
   Copy the printed magnet URI.

2. **Download in browser:**
   - Open `http://localhost:8080/demo/`
   - Go to the **Download** tab
   - Paste the magnet URI and click **Download**

The WebRTC connection is negotiated via the tracker; no direct TCP/UDP connectivity is needed between the Node seeder and browser leecher.

---

## Tracker Configuration

The tracker must have RtcTorrent support enabled in `config.toml`:

```toml
[[http_server]]
enabled       = true
bind_address  = "0.0.0.0:6969"
# ... other fields ...
rtctorrent    = true   # enable WebRTC signaling (default: false)
```

`rtctorrent = false` (the default) causes the tracker to return a bencode failure for any announce carrying `rtctorrent=1`, while standard BitTorrent announces are unaffected.

---

## How Signaling Works

Standard BitTorrent trackers only exchange IP addresses. RtcTorrent reuses the HTTP announce endpoint to exchange WebRTC SDP offers/answers:

| Step | Who | Announce params | Tracker response |
|------|-----|-----------------|-----------------|
| 1 | Seeder | `rtctorrent=1` + `rtcoffer=<SDP>` + `left=0` | stored, empty `rtc_peers` |
| 2 | Leecher | `rtctorrent=1` + `rtcrequest=1` + `left>0` | `rtc_peers` list with seeders' offers |
| 3 | Leecher | `rtctorrent=1` + `rtcanswer=<SDP>` + `rtcanswerfor=<seeder-peer-id>` | stored |
| 4 | Seeder (poll) | `rtctorrent=1` + `left=0` | `rtc_answers` list with pending answers |

After step 4 the WebRTC data channel opens and piece exchange begins directly peer-to-peer.

---

## API Reference

### `new RtcTorrent(options)`

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `trackerUrl` | `string` | — | HTTP announce URL (required) |
| `rtcInterval` | `number` | `10000` | Polling interval in ms for RTC announces |
| `iceServers` | `array` | Google STUN | ICE server list for WebRTC |

### `client.create(files, options)` → `{ infoHash, encodedTorrent, magnetUri }`

Creates torrent metadata. `files` may be `File` objects (browser) or file paths (Node.js).

### `client.seed(torrentData, files)`

Start seeding. `torrentData` is the encoded torrent buffer; `files` are paths (Node.js) or `File` objects (browser).

### `client.download(torrentData)` → `Torrent`

Start downloading. `torrentData` is a magnet URI string or encoded torrent buffer.

### `client.streamVideo(fileIndex, videoElement, options)`

*(Browser only)* Attach a `<video>` element to a downloading torrent for progressive playback. Supports faststart (web-optimized) MP4 files — playback starts after the `moov` atom and a short buffer are received, without waiting for the full download.

### `client.getBlob(fileOrIndex)` → `Blob`

Assemble all received pieces for a file into a `Blob`.

### Callbacks

- `client.onPieceReceived = (pieceIndex, data) => {}` — fires when each piece is fully received

---

## License

MIT
