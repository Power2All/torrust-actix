# RtcTorrent

A WebRTC-enabled BitTorrent client library. Seeders and leechers communicate over WebRTC data channels, using a standard HTTP BitTorrent tracker for signaling (SDP offer/answer exchange). Works in both **Node.js** and the **browser**.

---

## Prerequisites

- **torrust-actix tracker** running with `rtctorrent = true` in the `[[http_server]]` config block
- **Node.js** ≥ 12
- For Node.js WebRTC: `@roamhq/wrtc` native module — installed as an **optional** dependency; `npm install` succeeds on all platforms (Linux, macOS, Windows). If the native binary is unavailable on a given platform a warning is logged at runtime, but the browser bundle is unaffected.

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

Seed files directly from the command line (requires Node.js).

### Single-torrent mode

```
node bin/seed.js [OPTIONS] <file1> [<file2> ...]
```

| Flag | Default | Description |
|---|---|---|
| `<FILE>...` | — | One or more files to seed. Required unless `--torrent-file` is given. |
| `--tracker <URL>` | *(none)* | Tracker announce URL; repeat for multiple (BEP-12). Trackers are optional — omit to seed without announcing. |
| `--torrent-file <FILE>` | *(none)* | Re-seed from an existing `.torrent` — reads tracker URLs and info_hash from it. No re-hashing needed. |
| `--magnet <URI>` | *(none)* | Seed using a magnet URI — reads tracker URLs from it; files are still hashed from disk. |
| `--name <NAME>` | First file's filename | Torrent display name |
| `--out <PATH>` | `<name>.torrent` | Output path for the `.torrent` file |
| `--webseed <URL>` | *(none)* | WebSeed URL (BEP-19); repeat for multiple |
| `--torrent-version` | `v1` | Torrent format: `v1`, `v2`, or `hybrid` (see [BEP-52 section](#bep-52-bittorent-v2--hybrid-support)) |

**Examples:**

```bash
# Seed a single file (no tracker)
node bin/seed.js /path/to/movie.mp4

# Seed to a tracker
node bin/seed.js --tracker http://mytracker:6969/announce /path/to/movie.mp4

# Seed to multiple trackers (BEP-12)
node bin/seed.js \
  --tracker http://tracker1.example.com/announce \
  --tracker http://tracker2.example.com/announce \
  /path/to/movie.mp4

# Re-seed from an existing .torrent (no re-hashing)
node bin/seed.js --torrent-file movie.torrent
# Tracker URLs are read from the .torrent file automatically.

# Re-seed from a .torrent with an explicit file path
node bin/seed.js --torrent-file movie.torrent /data/movies/movie.mp4

# Seed using a magnet URI (tracker from magnet)
node bin/seed.js \
  --magnet "magnet:?xt=urn:btih:...&tr=http%3A%2F%2Ftracker.example.com%2Fannounce" \
  /path/to/movie.mp4

# Multi-file torrent with a custom name
node bin/seed.js --name "My Album" /music/track1.mp3 /music/track2.mp3

# Add an HTTP fallback (BEP-19 webseed)
node bin/seed.js --webseed https://cdn.example.com/movie.mp4 /path/to/movie.mp4
```

The seeder will:
1. Hash the file(s) and save a `.torrent` file (skipped when `--torrent-file` is given)
2. Print the magnet URI to share with leechers
3. Start seeding — pieces are read from disk on demand (no full file in RAM)

### Multi-torrent mode (YAML)

Seed multiple torrents concurrently from a single YAML config file:

```bash
node bin/seed.js --torrents /path/to/torrents.yaml
```

When `--torrents` is used, all single-torrent flags are forbidden.

**YAML format:**

```yaml
---
torrents:
  # Minimal — seed without announcing to any tracker
  - file:
      - "/data/movies/big_buck_bunny.mp4"

  # BEP-12 multi-tracker (all listed in announce-list)
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

| Field | Required | Description |
|---|---|---|
| `file` | **yes*** | List of local file paths to seed (*required unless `torrent_file` is set) |
| `trackers` | no | List of tracker announce URLs. If omitted, trackers are read from `torrent_file` or `magnet`. |
| `torrent_file` | no | Path to an existing `.torrent` file; tracker URLs and info_hash are read from it. |
| `magnet` | no | Magnet URI; tracker URLs are parsed from it. Files are still hashed from disk. |
| `out` | no | Output path for the `.torrent` file |
| `name` | no | Torrent display name |
| `webseed` | no | WebSeed (BEP-19) HTTP fallback URLs |
| `ice` | no | ICE server URLs (default: Google STUN) |
| `rtc_interval` | no | Announce poll interval in **milliseconds** (default: `5000`) |
| `version` | no | Torrent format: `"v1"` (default), `"v2"`, or `"hybrid"` |

**Reloading the config:**

| Platform | Trigger | How |
|---|---|---|
| Linux / macOS | `kill -HUP <pid>` | SIGHUP signal |
| Windows + all | Save the YAML file | File mtime polled every 2 s |

> **⚠️ Process exit on reload:** When a reload is triggered, the Node.js seeder
> prints a message and exits with code **0**. It does **not** reload in-process.
>
> **Why:** The `RtcTorrent` client has no public `destroy()` API, so there is no
> reliable way to stop the internal announce loops and peer connections without
> terminating the process.
>
> **What to do:** Use a process manager such as **pm2**, **systemd**, or **forever**
> so the process is restarted automatically after the clean exit:
>
> ```bash
> # pm2
> pm2 start bin/seed.js --name rtc-seed -- --torrents /etc/rtc-seed/torrents.yaml
> pm2 save
>
> # systemd — set Restart=on-success in the [Service] section
> ```
>
> If you need true in-process reload without a process manager, use the
> **Rust seeder** (`rtc-seed`) instead — it fully restarts tasks in-process.

---

## BEP-52 (BitTorrent v2 / Hybrid) Support

Both the CLI seeder (`seed.js`) and the library API (`client.create()`) support BitTorrent v2 (BEP-52) and hybrid torrent generation.

### `--torrent-version` flag (CLI)

```bash
# v1 — classic SHA-1 pieces (default, widest compatibility)
node bin/seed.js /path/to/movie.mp4

# v2 — pure BEP-52: SHA-256 Merkle trees, no SHA-1 pieces
node bin/seed.js --torrent-version v2 /path/to/movie.mp4

# hybrid — both SHA-1 and SHA-256 Merkle; magnet URI has two xt= params
node bin/seed.js --torrent-version hybrid /path/to/movie.mp4
```

### `version` field in the YAML config

```yaml
torrents:
  - file: ["/data/old_client.mp4"]
    trackers: ["http://tracker:6969/announce"]
    version: v1        # default

  - file: ["/data/new_content.mp4"]
    trackers: ["http://tracker:6969/announce"]
    version: hybrid    # SHA-1 + SHA-256 Merkle

  - file: ["/data/v2_only.mp4"]
    trackers: ["http://tracker:6969/announce"]
    version: v2        # pure BEP-52
```

### Format comparison

| | `v1` | `v2` | `hybrid` |
|---|---|---|---|
| Hash algorithm | SHA-1 (20 B) | SHA-256 Merkle (32 B) | Both |
| `info` dict | `pieces`, `files`/`length` | `file tree`, `meta version` | All fields |
| Top-level key | — | `piece layers` | `piece layers` |
| Magnet URI | `xt=urn:btih:…` | `xt=urn:btmh:1220…` | Both `xt=` params |
| Tracker announce hash | SHA-1 20 B | SHA-256 first 20 B | SHA-1 20 B |
| v1 client compat | ✓ | ✗ | ✓ |
| v2 client compat | ✗ | ✓ | ✓ |

### Leecher / download side

- `parseMagnet()` detects `xt=urn:btmh:1220…` automatically
- `parseTorrentFile()` detects v2/hybrid by the presence of `file tree` / `meta version: 2`
- SHA-256 Merkle piece verification is **not yet implemented** (pure v2 pieces are accepted without hash check — future work)

---

## Rust Native Seeder (`rtc-seed`)

A pure-Rust alternative to the Node.js seeder — no Node.js required. Located in `lib/rtc-seed/`.

### Build

```bash
# From the repository root
cargo build -p rtc-seed --release
```

### Usage

```bash
# Single-torrent
rtc-seed [--tracker <url>] ... [--torrent-file <path>] [--magnet <uri>] \
         [--name <name>] [--out <file.torrent>] [--webseed <url>] [--ice <url>] \
         <file1> [<file2> ...]

# Multi-torrent (same YAML format as the Node.js seeder above)
rtc-seed --torrents /path/to/torrents.yaml
```

**Examples:**

```bash
# Seed a single file (no tracker)
rtc-seed /path/to/movie.mp4

# Seed to a tracker
rtc-seed --tracker http://mytracker:6969/announce /path/to/movie.mp4

# Seed to multiple trackers (BEP-12)
rtc-seed \
  --tracker http://tracker1.example.com/announce \
  --tracker http://tracker2.example.com/announce \
  /path/to/movie.mp4

# Re-seed from an existing .torrent (no re-hashing)
rtc-seed --torrent-file movie.torrent

# Seed using a magnet URI (tracker from magnet)
rtc-seed --magnet "magnet:?xt=urn:btih:...&tr=..." /path/to/movie.mp4

# Seed multiple torrents from a YAML file
rtc-seed --torrents /etc/rtc-seed/torrents.yaml

# Run directly from the workspace without installing
cargo run -p rtc-seed -- --tracker http://127.0.0.1:6969/announce /path/to/movie.mp4
cargo run -p rtc-seed -- --torrents /path/to/torrents.yaml
```

In multi-torrent mode the Rust seeder performs **true in-process reload** — the process
stays alive and all seeder tasks are restarted without any process exit:

| Platform | Trigger | Result |
|---|---|---|
| Linux / macOS | `kill -HUP <pid>` | Tasks abort, YAML re-read, new tasks start |
| Windows + all | Save the YAML file | Same — file mtime polled every 2 s |
| Any | `Ctrl+C` | Clean shutdown |

This is possible because each torrent runs as a `tokio::spawn` task that can be
`.abort()`ed cleanly. No process manager is required for reload. This is the key
advantage over the Node.js seeder, which must exit and be restarted by a process manager.

Expected output:

```
=== RtcTorrent Seeder (Rust native) ===
Tracker : http://127.0.0.1:6969/announce
Files   : /path/to/movie.mp4

Creating torrent (hashing pieces)… done.

Saved : movie.torrent
Hash  : 35a3e807d020...

Magnet URI:
magnet:?xt=urn:btih:35a3e807d020...&dn=movie&tr=http%3A%2F%2F127.0.0.1%3A6969%2Fannounce

Share the magnet URI or the .torrent file with leechers.

Creating WebRTC offer (gathering ICE candidates)… done.
Seeding… (Ctrl+C to stop)
```

---

## ⚠️ Localhost Testing — WebRTC mDNS Warning

Modern browsers (Chrome, Edge, Firefox) hide local IP addresses in WebRTC ICE candidates by replacing them with `.local` mDNS hostnames (e.g. `abc123.local`). This is a privacy feature that works fine in production (where a TURN/STUN relay resolves the addresses), but **breaks direct WebRTC connections on localhost** because the Rust seeder (`rtc-seed`) cannot resolve mDNS hostnames.

Symptom: the data channel never opens, and you see log messages like:
```
discard success message from (172.x.x.x:port), no such remote
peer connection state changed: failed
```

### Fix per browser

**Chrome / Edge — command line:**
```bash
# Chrome
chrome --disable-features=WebRtcHideLocalIpsWithMdns

# Edge (Windows)
msedge --disable-features=WebRtcHideLocalIpsWithMdns

# Edge — full path if needed
"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe" --disable-features=WebRtcHideLocalIpsWithMdns
```

**Chrome / Edge — flags page (no restart of system needed):**
1. Open `chrome://flags` or `edge://flags`
2. Search for **"Anonymize local IPs exposed by WebRTC"**
3. Set to **Disabled**
4. Click **Restart**

**Firefox:**
1. Open `about:config`
2. Search for `media.peerconnection.ice.obfuscate_host_candidates`
3. Set to **`false`**
4. Restart Firefox

> **Note:** This only affects local/LAN testing. On a public server with a TURN relay the browser's default behaviour works correctly and no flag change is needed.

---

## Browser Demo (`demo/index.html`)

A full browser UI to seed and download torrents. Uses **[Video.js 8.23.4](https://videojs.com/)** for video playback (loaded from CDN — no extra install needed).

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
3. Playback begins automatically for the first playable file found using the Video.js player:
   - **Service Worker active:** progressive streaming — video plays as pieces arrive; once fully downloaded the source switches to a Blob URL (brief pause) to enable seeking
   - **SW not available:** progressive blob streaming or MSE depending on format
4. Use **▶ Play** on any file in the list to switch, or **↓ Save** to download to disk

---

## Embeddable Player (`demo/player.html`)

A standalone full-page player powered by **Video.js 8.23.4** (loaded from CDN). Link to it directly:

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

Drop a self-contained video player into any page with two script tags and a single JS call. Video.js 8.23.4 is loaded automatically from CDN — no extra dependencies to install.

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

# Or re-seed from an existing .torrent (no re-hashing):
node bin/seed.js --torrent-file movie.torrent

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

### `client.create(files, options)` → `{ infoHash, encodedTorrent, magnetUri, v2InfoHash? }`

Creates torrent metadata and returns the encoded `.torrent` buffer plus magnet URI.

- **`files`** — `File[]` objects (browser) or file path strings (Node.js)
- **`options.name`** — torrent display name
- **`options.version`** — torrent format: `'v1'` *(default)*, `'v2'`, or `'hybrid'` (see [BEP-52 section](#bep-52-bittorent-v2--hybrid-support) below)
- **`options.trackerUrl`** — announce URL to embed in the torrent
- **`options.webseedUrls`** — string or string array of HTTP fallback URLs (BEP-19 `url-list`). For single-file torrents: direct URL to the file. For multi-file torrents: base directory URL (trailing slash optional).

For v2/hybrid, the returned object also includes `v2InfoHash` (64-character hex SHA-256).

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
torrent.onSeekReady        = () => { /* seeking is now available (blob switch complete) */ };
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
