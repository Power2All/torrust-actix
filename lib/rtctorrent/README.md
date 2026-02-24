# RtcTorrent

A WebRTC-enabled BitTorrent client library. Seeders and leechers communicate over WebRTC data channels, using a standard HTTP BitTorrent tracker for signaling (SDP offer/answer exchange). Works in both **Node.js** and the **browser**.

---

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

## Tests

Two automated tests verify the full stack without a browser.

### 1. Signaling flow test

Validates the tracker's offer/answer signaling (no WebRTC hardware required):

```bash
node test/test_signaling_flow.js
# Expected: 13 passed, 0 failed
```

### 2. End-to-end WebRTC transfer test

Creates a real temp file, seeds it from one Node process, downloads it in another, and byte-checks the result:

```bash
# Tracker must be running first
node test/test_webrtc_transfer.js
# or: npm run test:transfer
```

Expected output:
```
=== WebRTC Transfer Test ===
Test file: /tmp/rtctest_<timestamp>.bin  (5600 bytes)
[Seeder] SDP offer created (... bytes)
[Leecher] Data channel opened
[Leecher] Piece 0 OK (5600 B) — 1/1 pieces
[LEECHER] Download complete!
Data match: PASS ✓
```

---

## Node.js CLI Seeder

Seed files directly from the command line:

```bash
node bin/seed.js [--tracker <url>] [--name <name>] [--out <file.torrent>] [--webseed <url>] <file1> [<file2> ...]
```

**Examples:**

```bash
# Seed a single file using the default tracker
node bin/seed.js /path/to/movie.mp4

# Custom tracker, custom output name
node bin/seed.js --tracker http://mytracker:6969/announce --name "My Movie" /path/to/movie.mp4

# Multi-file torrent
node bin/seed.js --name "My Album" /music/track1.mp3 /music/track2.mp3

# Add an HTTP fallback (BEP-19 webseed) so leechers can download even without WebRTC peers
node bin/seed.js --webseed https://cdn.example.com/movie.mp4 /path/to/movie.mp4

# Multiple webseed URLs
node bin/seed.js --webseed https://cdn1.example.com/movie.mp4 --webseed https://cdn2.example.com/movie.mp4 /path/to/movie.mp4
```

The seeder will:
1. Hash the file(s) and save a `.torrent` file next to the script
2. Print the magnet URI to share with leechers
3. Start seeding — pieces are read from disk on demand (no full file in RAM)

```
=== RtcTorrent Seeder ===
Tracker : http://127.0.0.1:6969/announce
Files   : /path/to/movie.mp4

Creating torrent (hashing pieces)… done.

Saved : movie.torrent
Hash  : 35a3e807d020...

Magnet URI:
magnet:?xt=urn:btih:35a3e807d020...&dn=movie&tr=http%3A%2F%2F127.0.0.1%3A6969%2Fannounce

Seeding… (Ctrl+C to stop)
```

---

## Browser Demo (`demo/index.html`)

A full browser UI to seed and download torrents.

### Start the demo server

```bash
npm run serve
# or: node bin/serve.js [port]   (default: 8080)
```

Open **http://localhost:8080/demo/** in your browser.

### Seed tab

**Option A — Seed an existing `.torrent`**
1. Drop or click to load a `.torrent` file
2. Select the matching data file(s) — the page validates names and sizes
3. Click **Create & Seed** — seeding starts immediately

**Option B — Create a new torrent from scratch**
1. Click **Select Files / Folder**
2. Optionally set a torrent name
3. Click **Create & Seed** — the torrent is created on the fly and the `.torrent` file is auto-downloaded

### Download tab

1. Paste a magnet URI **or** drop/click to load a `.torrent` file
2. Click **Download**
3. Playback begins automatically for the first playable file found:
   - **Service Worker active:** fully seamless streaming — video plays as pieces arrive, no reloads
   - **SW not available:** progressive blob streaming or MSE depending on format
4. Use **▶ Play** on any file in the list to switch, or **↓ Save** to download to disk

---

## Embeddable Player (`demo/player.html`)

A standalone full-page player you can link to directly:

```
http://localhost:8080/demo/player.html?torrent=https://cdn.example.com/file.torrent&tracker=http://tracker:6969/announce
http://localhost:8080/demo/player.html?magnet=magnet:?xt=urn:btih:...&tracker=http://tracker:6969/announce&file=0
```

Query parameters:

| Parameter | Description | Default |
|-----------|-------------|---------|
| `torrent` | URL to a `.torrent` file | — |
| `magnet`  | `magnet:` URI | — |
| `tracker` | Tracker announce URL | `http://127.0.0.1:6969/announce` |
| `file`    | File index inside the torrent | `0` |

---

## Embeddable Widget (`demo/embed.js`)

Drop a self-contained video player into any page with two script tags and a single JS call.

### Usage

```html
<!-- 1. A container with an explicit size -->
<div id="player" style="width:100%; aspect-ratio:16/9"></div>

<!-- 2. Library + embed script -->
<script src="dist/rtctorrent.browser.js"></script>
<script src="demo/embed.js"></script>

<!-- 3. Create the player -->
<script>
  RtcTorrentPlayer('#player', {
    torrent:  'https://cdn.example.com/file.torrent',
    tracker:  'https://tracker.example.com/announce',
    swPath:   '/sw.js',   // copy sw.js to your server root for seamless streaming
  });
</script>
```

### All options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `torrent` | string | — | URL to a `.torrent` file *(required if no `magnet`)* |
| `magnet` | string | — | `magnet:` URI *(required if no `torrent`)* |
| `tracker` | string | `http://127.0.0.1:6969/announce` | Announce URL |
| `file` | number | `0` | File index inside the torrent |
| `swPath` | string | `null` | Path to `sw.js` on your server — enables seamless SW streaming |
| `autoplay` | boolean | `true` | Autoplay when buffered |
| `muted` | boolean | `false` | Start muted (required for autoplay in most browsers) |
| `initialExtra` | number | `50` | Extra pieces to buffer before playback starts (~3 MB at 64 KB/piece) |
| `rtcInterval` | number | `5000` | RTC announce poll interval (ms) |
| `iceServers` | array | Google STUN | WebRTC ICE server list |
| `statsInterval` | number | `5000` | Console stats log interval (ms) |
| `onStats` | function | — | Called each stats tick: `({ downloaded, total, percent, pieces, totalPieces, peers, speed })` |
| `onError` | function | — | Called on fatal error with the message string |
| `onReady` | function | — | Called when torrent metadata is ready, receives the player instance |

### Instance methods

```js
const player = RtcTorrentPlayer('#player', { ... });

player.destroy();   // stop streaming and remove the player from the DOM
```

### Console output

While buffering, stats are printed every `statsInterval` ms:

```
[RtcTorrentPlayer] ▶ "movie.mp4"  |  263.2 MB  |  4214 pieces × 64 KB  |  hash: 35a3e8…
[RtcTorrentPlayer] Stats:  12.3 MB / 263.2 MB  (4.7%)  |  196/4214 pieces  |  3 peers  |  ↓ 512.0 KB/s
[RtcTorrentPlayer] ✓ Complete:  263.2 MB / 263.2 MB  (100.0%)  |  4214/4214 pieces  |  2 peers
```

---

## Service Worker Setup (Seamless Streaming)

Without a Service Worker the player falls back to blob/MSE streaming, which works but may stall briefly when new data arrives. For truly seamless video playback (no interruptions):

1. Copy `demo/sw.js` to the **root** of your web server (e.g. `/sw.js`)
2. Set `swPath: '/sw.js'` in the embed options, or register it manually:
   ```js
   navigator.serviceWorker.register('/sw.js');
   ```
3. The SW intercepts `/__rtc_stream__/*` URLs and serves a live `ReadableStream` to the `<video>` element as pieces arrive

> The SW must be on the same origin as the page (browser security requirement).

---

## Node Seed → Browser Leech (typical workflow)

```bash
# 1. Start the tracker
./target/debug/torrust-actix

# 2. Serve the demo
cd lib/rtctorrent && npm run serve

# 3. Seed a file from Node
node bin/seed.js --tracker http://127.0.0.1:6969/announce /path/to/movie.mp4
# → copy the printed magnet URI

# 4. Open the demo in your browser
#    http://localhost:8080/demo/
#    → Download tab → paste magnet URI → Download
```

---

## Tracker Configuration

Enable RtcTorrent in `config.toml`:

```toml
[[http_server]]
enabled       = true
bind_address  = "0.0.0.0:6969"
rtctorrent    = true   # enable WebRTC signaling (default: false)
```

`rtctorrent = false` (the default) causes the tracker to return a bencode failure for any announce carrying `rtctorrent=1`, while standard BitTorrent announces are unaffected.

---

## How Signaling Works

Standard BitTorrent trackers only exchange IP addresses. RtcTorrent reuses the HTTP announce endpoint to exchange WebRTC SDP offers/answers:

| Step | Who | Announce params | Tracker response |
|------|-----|-----------------|------------------|
| 1 | Seeder | `rtctorrent=1` + `rtcoffer=<SDP>` + `left=0` | stored, empty `rtc_peers` |
| 2 | Leecher | `rtctorrent=1` + `rtcrequest=1` + `left>0` | `rtc_peers` list with seeders' offers |
| 3 | Leecher | `rtctorrent=1` + `rtcanswer=<SDP>` + `rtcanswerfor=<seeder-peer-id>` | stored |
| 4 | Seeder (poll) | `rtctorrent=1` + `left=0` | `rtc_answers` list with pending answers |

After step 4 the WebRTC data channel opens and piece exchange begins directly peer-to-peer.

---

## API Reference

### `new RtcTorrent(options)`

```js
const client = new RtcTorrent({
  trackerUrl:  'http://tracker:6969/announce',  // required
  rtcInterval: 5000,        // RTC poll interval in ms (default: 10000)
  iceServers:  [...],       // WebRTC ICE servers (default: Google STUN)
});
```

---

### `client.create(files, options)` → `{ infoHash, encodedTorrent, magnetUri }`

Creates torrent metadata and returns the encoded `.torrent` buffer plus magnet URI.

- **`files`** — `File[]` objects (browser) or file path strings (Node.js)
- **`options.name`** — torrent display name
- **`options.trackerUrl`** — announce URL to embed in the torrent
- **`options.webseedUrls`** — string or string array of HTTP fallback URLs (BEP-19 `url-list`). For single-file torrents: direct URL to the file. For multi-file torrents: base directory URL (trailing slash optional).

---

### `client.seed(torrentData, files)` → `Torrent`

Start seeding.

- **`torrentData`** — encoded torrent `Buffer`/`Uint8Array`
- **`files`** — `File[]` objects (browser) or file path strings (Node.js)

Returns a `Torrent` instance (see below).

---

### `client.download(torrentData)` → `Torrent`

Start downloading.

- **`torrentData`** — magnet URI string, `.torrent` URL string, or encoded `Uint8Array`

Returns a `Torrent` instance (see below).

---

### `client.parseTorrentFile(bytes)` → parsed info

Parse a `.torrent` file without starting a download. Returns the raw decoded torrent object including `infoHash`, `name`, and `info`.

---

### `client.stop()` → `Promise`

Stop all torrents and close all WebRTC connections.

---

### Torrent instance

Returned by both `seed()` and `download()`.

#### Properties

| Property | Type | Description |
|----------|------|-------------|
| `totalSize` | number | Total byte size of all files |
| `downloaded` | number | Bytes received so far |
| `pieceCount` | number | Total number of pieces |
| `pieceLength` | number | Bytes per piece |
| `pieces` | `Map<number, Uint8Array>` | Received pieces |
| `peers` | `Map<string, Peer>` | Active WebRTC peers |
| `files` | array | `[{ name, length, offset }]` |
| `active` | boolean | Set to `false` to stop all activity |
| `webseeds` | string[] | HTTP fallback URLs parsed from `url-list` in the torrent file |

#### Callbacks

```js
torrent.onDownloadComplete = () => { /* all pieces received */ };
torrent.onPieceReceived    = (pieceIndex, data) => { /* one piece arrived */ };
```

#### `torrent.streamVideo(fileIndex, videoElement, options)` *(browser only)*

Attach a `<video>` element for progressive playback. Streaming modes are tried in order:

1. **Service Worker** — seamless, no reloads (requires `sw.js` registered on the page)
2. **MSE** — Media Source Extensions for fragmented MP4/WebM
3. **Faststart blob** — progressive blob rebuild for web-optimized MP4
4. **Full blob** — waits for all pieces, then plays from a blob URL

```js
await torrent.streamVideo(0, document.getElementById('v'), {
  onProgress:   pct => console.log(`Buffering ${pct}%`),
  initialExtra: 50,   // pieces to buffer before playback (default: 50 ≈ 3 MB)
});
```

#### `torrent.getBlob(fileOrIndex)` → `Blob` *(browser only)*

Assemble all received pieces for a file into a `Blob` for download or playback.

#### `torrent.saveFile(fileIndex)` *(browser only)*

Trigger a browser download of the completed file.

#### `torrent.stop()`

Stop this torrent and close its peers.

---

## License

MIT
