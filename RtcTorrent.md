# RtcTorrent Protocol — White Paper

**Version:** 4.2.0
**Authors:** Torrust-Actix Project
**Status:** Reference Implementation

---

## Table of Contents

1. [Abstract](#1-abstract)
2. [Background and Motivation](#2-background-and-motivation)
3. [Design Goals](#3-design-goals)
4. [Protocol Architecture Overview](#4-protocol-architecture-overview)
5. [Tracker Announce Extensions](#5-tracker-announce-extensions)
   - 5.1 [New Query Parameters](#51-new-query-parameters)
   - 5.2 [Tracker Response Fields](#52-tracker-response-fields)
   - 5.3 [Error Handling](#53-error-handling)
6. [Signaling Flow — Step by Step](#6-signaling-flow--step-by-step)
   - 6.1 [Step 1: Seeder Registers an Offer](#61-step-1-seeder-registers-an-offer)
   - 6.2 [Step 2: Leecher Requests Offers](#62-step-2-leecher-requests-offers)
   - 6.3 [Step 3: Leecher Submits Its Answer](#63-step-3-leecher-submits-its-answer)
   - 6.4 [Step 4: Seeder Polls for Answers](#64-step-4-seeder-polls-for-answers)
7. [Tracker-Side Data Model](#7-tracker-side-data-model)
   - 7.1 [Peer Entry Structure](#71-peer-entry-structure)
   - 7.2 [Torrent Entry Structure](#72-torrent-entry-structure)
   - 7.3 [Answer Queue Preservation](#73-answer-queue-preservation)
8. [WebRTC Data Channel Protocol](#8-webrtc-data-channel-protocol)
   - 8.1 [Channel Configuration](#81-channel-configuration)
   - 8.2 [Message Types](#82-message-types)
   - 8.3 [MSG_PIECE_REQUEST (0x01)](#83-msg_piece_request-0x01)
   - 8.4 [MSG_PIECE_DATA (0x02)](#84-msg_piece_data-0x02)
   - 8.5 [MSG_PIECE_CHUNK (0x04)](#85-msg_piece_chunk-0x04)
   - 8.6 [Chunked Transfer for Large Pieces](#86-chunked-transfer-for-large-pieces)
   - 8.7 [Flow Control](#87-flow-control)
9. [Client-Side Implementation Guide](#9-client-side-implementation-guide)
   - 9.1 [Peer Identity](#91-peer-identity)
   - 9.2 [ICE and SDP Lifecycle](#92-ice-and-sdp-lifecycle)
   - 9.3 [Announce Loop](#93-announce-loop)
   - 9.4 [In-Flight Request Management](#94-in-flight-request-management)
   - 9.5 [Piece Verification](#95-piece-verification)
   - 9.6 [Peer Speed Monitoring and Blacklisting](#96-peer-speed-monitoring-and-blacklisting)
   - 9.7 [WebSeed Fallback (BEP-19)](#97-webseed-fallback-bep-19)
10. [Torrent Format Support](#10-torrent-format-support)
    - 10.1 [BitTorrent v1 (SHA-1)](#101-bittorrent-v1-sha-1)
    - 10.2 [BitTorrent v2 (BEP-52, SHA-256 Merkle)](#102-bittorrent-v2-bep-52-sha-256-merkle)
    - 10.3 [Hybrid Torrents](#103-hybrid-torrents)
11. [Tracker Implementation Guide](#11-tracker-implementation-guide)
    - 11.1 [Parsing Announce Requests](#111-parsing-announce-requests)
    - 11.2 [Storing Peers](#112-storing-peers)
    - 11.3 [Building the RTC Response](#113-building-the-rtc-response)
    - 11.4 [Answer Queue Atomicity](#114-answer-queue-atomicity)
    - 11.5 [Filtering Peers](#115-filtering-peers)
    - 11.6 [Configuration](#116-configuration)
12. [URL Encoding Requirements](#12-url-encoding-requirements)
13. [Browser Compatibility and CORS](#13-browser-compatibility-and-cors)
14. [Known Pitfalls and Critical Implementation Notes](#14-known-pitfalls-and-critical-implementation-notes)
15. [Interoperability with Standard BitTorrent](#15-interoperability-with-standard-bittorrent)
16. [Security Considerations](#16-security-considerations)
17. [Reference Implementation Summary](#17-reference-implementation-summary)
18. [Glossary](#18-glossary)

---

## 1. Abstract

**RtcTorrent** is a protocol extension to the standard BitTorrent HTTP tracker announce mechanism that enables browser-native WebRTC peer-to-peer data transfer using existing tracker infrastructure. No WebSocket server, no new server component, and no changes to the .torrent file format are required. The tracker's existing `/announce` endpoint is reused as an out-of-band signaling channel: seeders publish SDP offers, leechers retrieve those offers and submit SDP answers, and seeders poll for answers on their next announce. Once ICE negotiation completes, peers communicate over a WebRTC unreliable datagram channel using a simple binary piece-exchange protocol.

---

## 2. Background and Motivation

Traditional BitTorrent operates over TCP or UDP connections. Browsers cannot open raw TCP/UDP sockets, making BitTorrent inaccessible from web applications without a local native client. WebRTC's `RTCDataChannel` provides browser-native peer-to-peer binary data transfer, but WebRTC requires an out-of-band signaling step (SDP offer/answer exchange) before the peer connection can be established.

Existing WebTorrent-family implementations solve this by deploying a separate WebSocket-based tracker. RtcTorrent takes a different approach: it **piggybacks the signaling step onto the existing HTTP tracker announce**, adding four optional query parameters. This means:

- **No new server infrastructure** is needed — any HTTP tracker that adds support for the four RTC parameters can become a WebRTC signaling relay.
- **Backward compatibility** is maintained — non-RTC clients see normal announce responses, while RTC clients send and receive the additional fields.
- **Multi-tracker support** works transparently — a torrent can list both RTC-capable and non-RTC trackers; the client skips RTC signaling for trackers that return an error or omit the RTC response fields.

---

## 3. Design Goals

| Goal | Approach |
|------|----------|
| No new server infrastructure | Reuse `/announce` endpoint for signaling |
| Browser-native operation | WebRTC DataChannel, no raw socket access required |
| Backward compatible | RTC parameters are optional; non-RTC peers are unaffected |
| Stateless signaling relay | Tracker stores offer/answer as peer metadata, not session state |
| Cross-environment | Same JS library runs in browsers and Node.js |
| Standard torrent formats | Support BT v1, v2 (BEP-52), and hybrid torrents |
| Graceful degradation | Falls back to WebSeed HTTP (BEP-19) if no WebRTC peers available |

---

## 4. Protocol Architecture Overview

```
  SEEDER                    TRACKER (/announce)               LEECHER
    |                             |                              |
    |-- announce + rtcoffer ----->|                              |
    |   (registers SDP offer)     |                              |
    |                             |<-- announce + rtcrequest ----|
    |                             |    (requests offers)         |
    |                             |-- rtc_peers list ----------->|
    |                             |   (seeder peer_id + offer)   |
    |                             |                              |
    |                             |<-- announce + rtcanswer -----|
    |                             |    rtcanswerfor=<seeder_id>  |
    |                             |    (deposits SDP answer)     |
    |                             |                              |
    |<-- announce + rtc_answers --|                              |
    |    (picks up SDP answer)    |                              |
    |                             |                              |
    |<============= WebRTC DataChannel (direct P2P) ==========>|
    |    MSG_PIECE_REQUEST        |                              |
    |    MSG_PIECE_DATA           |                              |
    |    MSG_PIECE_CHUNK          |                              |
```

The tracker acts purely as a **mailbox**: it stores the seeder's SDP offer in the peer's record and stores leecher SDP answers in a pending-answer queue attached to the target seeder's peer record. The tracker never interprets or validates the SDP content.

---

## 5. Tracker Announce Extensions

### 5.1 New Query Parameters

All parameters are optional from the tracker's perspective. Their presence activates RTC-specific behavior.

| Parameter | Type | Description |
|-----------|------|-------------|
| `rtctorrent` | integer `1` | Signals that this announce is RTC-capable. Must be present for any RTC operation. |
| `rtcoffer` | string | URL-encoded SDP offer from a seeder. Only sent by seeders (`left=0`). |
| `rtcrequest` | integer `1` | Leecher flag indicating it wants a list of seeders with pending offers. |
| `rtcanswer` | string | URL-encoded SDP answer from a leecher, directed at a specific seeder. |
| `rtcanswerfor` | string | The target seeder's `peer_id` encoded as a 40-character lowercase hex string (20 bytes). Required when `rtcanswer` is present. |

All five parameters co-exist with the standard announce parameters (`info_hash`, `peer_id`, `port`, `uploaded`, `downloaded`, `left`, `compact`, `event`, `numwant`).

**Example — Seeder announce with offer:**
```
GET /announce?info_hash=%ab%cd...&peer_id=-RT1000-123456789012&port=6881
  &uploaded=0&downloaded=0&left=0&compact=1
  &rtctorrent=1&rtcoffer=v%3D0%0D%0Ao%3D-+... (URL-encoded SDP)
```

**Example — Leecher requesting offers:**
```
GET /announce?info_hash=%ab%cd...&peer_id=-RT1000-987654321098&port=6881
  &uploaded=0&downloaded=0&left=65536&compact=1
  &rtctorrent=1&rtcrequest=1
```

**Example — Leecher submitting answer:**
```
GET /announce?info_hash=%ab%cd...&peer_id=-RT1000-987654321098&port=6881
  &uploaded=0&downloaded=0&left=65536&compact=1
  &rtctorrent=1&rtcanswer=v%3D0%0D%0Ao%3D...&rtcanswerfor=abcd1234...
```

### 5.2 Tracker Response Fields

When `rtctorrent=1` is present in the request, the tracker returns a **different bencoded response** than usual:

**RTC-specific response (returned instead of the normal peers list):**

```bencode
d
  11:rtc interval i10000e
  8:complete    i<seed_count>e
  10:incomplete i<peer_count>e
  9:rtc_peers   l
    d
      7:peer_id  20:<20-byte binary peer_id>
      9:sdp_offer <length>:<SDP offer string>
    e
    ...
  e
  11:rtc_answers l
    d
      7:peer_id   20:<20-byte binary peer_id of the leecher>
      10:sdp_answer <length>:<SDP answer string>
    e
    ...
  e
e
```

Field descriptions:

| Field | Type | Description |
|-------|------|-------------|
| `rtc interval` | integer | Milliseconds between announce cycles. Clients must respect this. |
| `complete` | integer | Number of complete RTC peers (seeders). |
| `incomplete` | integer | Number of incomplete RTC peers (leechers). |
| `rtc_peers` | list | Seeders with pending SDP offers. Returned to leechers. Each entry contains a binary `peer_id` (20 bytes) and `sdp_offer` string. |
| `rtc_answers` | list | SDP answers deposited by leechers, addressed to this peer. Returned to seeders. Each entry contains a binary `peer_id` (20 bytes, the leecher's identity) and `sdp_answer` string. |

**Important:** `rtc_peers` and `rtc_answers` are **mutually targeted** — `rtc_peers` is only useful to leechers (it lists seeders' offers), and `rtc_answers` is only useful to seeders (it lists answers waiting for them). The tracker returns both in every RTC response, but the non-applicable list will be empty.

**Normal (non-RTC) responses** also include `rtc interval` as a hint for the client to schedule its next RTC announce:

```bencode
d
  8:interval     i1800e
  12:min interval i60e
  12:rtc interval i10000e
  8:complete    i<n>e
  10:incomplete i<n>e
  10:downloaded i<n>e
  5:peers       <compact binary>
e
```

### 5.3 Error Handling

If the tracker does not support RTC signaling (the `rtctorrent` parameter is enabled but not configured), it returns:

```bencode
d 14:failure reason 22:rtctorrent not enabled e
```

Clients detecting a `failure reason` that contains the substring `rtctorrent` (case-insensitive) should add that tracker to a temporary ignore list and stop sending RTC announces to it. The ignore list is session-scoped; it resets on restart.

If neither `rtc_peers` nor `rtc_answers` appears in a successful response, the client similarly treats the tracker as RTC-incapable.

---

## 6. Signaling Flow — Step by Step

### 6.1 Step 1: Seeder Registers an Offer

The seeder creates an `RTCPeerConnection`, opens a data channel on it, generates an SDP offer, and waits for ICE candidate gathering to complete (up to 5 seconds). It then encodes the final SDP (with ICE candidates embedded) and announces it to the tracker.

```
GET /announce?...&left=0&rtctorrent=1&rtcoffer=<url-encoded-SDP>
```

The tracker stores the SDP offer string in the peer's record under `rtc_sdp_offer` and marks `rtc_connection_status = "offered"`. The peer is inserted into the `rtc_seeds` map (because `left=0`).

The seeder re-announces periodically (driven by `rtc interval`). On each re-announce it sends the same (cached) SDP offer and receives any `rtc_answers` that leechers have deposited since the last poll. This is the seeder's only mechanism to discover that a leecher wants to connect.

### 6.2 Step 2: Leecher Requests Offers

A leecher that wants to connect announces with `rtcrequest=1` and `left > 0`:

```
GET /announce?...&left=65536&rtctorrent=1&rtcrequest=1
```

The tracker returns a list of all seeders' peer_ids and their SDP offers in `rtc_peers` (excluding the requesting peer if it happens to also be present). The leecher iterates this list.

### 6.3 Step 3: Leecher Submits Its Answer

For each seeder offer received, the leecher creates its own `RTCPeerConnection`, sets the seeder's SDP offer as the remote description, generates an SDP answer, gathers ICE candidates, then deposits the answer back to the tracker via a second announce:

```
GET /announce?...&left=65536&rtctorrent=1
  &rtcanswer=<url-encoded-SDP-answer>
  &rtcanswerfor=<seeder-peer-id-hex>
```

The `rtcanswerfor` value is the seeder's `peer_id` as a 40-character lowercase hex string (the binary 20-byte `peer_id` from the `rtc_peers` response, hex-encoded by the client).

The tracker decodes the hex string back to 20 bytes, locates the target seeder's peer record, and appends `(leecher_peer_id, sdp_answer)` to that seeder's `rtc_pending_answers` vector.

**Important:** The answer announce is a separate HTTP request from the request-offers announce. The leecher sends two announces in rapid succession during setup: one to get offers, one to deposit its answer.

### 6.4 Step 4: Seeder Polls for Answers

On its next periodic announce, the seeder sends its SDP offer again. The tracker's response now includes the leecher's SDP answer in `rtc_answers`. Crucially, the tracker uses an **atomic take** operation: `std::mem::take` removes and returns the `rtc_pending_answers` vector in a single write-locked operation, ensuring that each answer is delivered exactly once.

The seeder calls `setRemoteDescription({ type: 'answer', sdp: answerInfo.sdp_answer })` on its existing `RTCPeerConnection`. If ICE negotiation succeeds, the WebRTC connection is established and the data channel opens.

---

## 7. Tracker-Side Data Model

### 7.1 Peer Entry Structure

Each peer in the tracker's store has the following RTC-relevant fields (in addition to standard fields like `peer_id`, `peer_addr`, `uploaded`, `downloaded`, `left`):

```rust
pub struct TorrentPeer {
    // ... standard fields ...
    pub is_rtctorrent: bool,
    pub rtc_sdp_offer: Option<String>,
    pub rtc_sdp_answer: Option<String>,
    pub rtc_connection_status: String, // "pending" | "offered" | "connected"
    pub rtc_pending_answers: Vec<(PeerId, String)>, // (answerer_peer_id, sdp_answer)
}
```

- `is_rtctorrent`: Set to `true` if the announce included `rtctorrent=1`. Used to route the peer into `rtc_seeds` or `rtc_peers` rather than standard `seeds` or `peers`.
- `rtc_sdp_offer`: The seeder's SDP offer string. Set when `rtcoffer` is present.
- `rtc_pending_answers`: A queue of `(peer_id, sdp_answer)` pairs deposited by leechers. **Atomically drained** when the seeder polls.
- `rtc_connection_status`: Lifecycle tracking: `"pending"` → `"offered"` → `"connected"`.

### 7.2 Torrent Entry Structure

Each torrent entry holds four peer maps:

```rust
pub struct TorrentEntry {
    pub seeds: AHashMap<PeerId, TorrentPeer>,     // non-RTC seeders
    pub peers: AHashMap<PeerId, TorrentPeer>,     // non-RTC leechers
    pub rtc_seeds: AHashMap<PeerId, TorrentPeer>, // RTC seeders (left=0)
    pub rtc_peers: AHashMap<PeerId, TorrentPeer>, // RTC leechers (left>0)
    pub completed: u32,
    pub updated: std::time::Instant,
}
```

RTC peers are tracked separately from traditional peers. This separation:
- Allows RTC-only response generation without filtering a combined list
- Allows `complete`/`incomplete` counts for RTC to be reported separately
- Preserves the tracker's existing non-RTC functionality unchanged

### 7.3 Answer Queue Preservation

A critical implementation correctness requirement: when a seeder re-announces (which calls `add_torrent_peer` to update its peer record), the existing `rtc_pending_answers` **must not be discarded**.

The naive implementation that removes and re-inserts a peer entry will silently drop any answers deposited between the last poll and the re-announce. The correct implementation captures the existing answers before removing the old peer record and restores them into the newly inserted record:

```rust
// In add_torrent_peer, Entry::Occupied branch:
let old_rtc_pending_answers = entry.rtc_seeds.get(&peer_id)
    .or_else(|| entry.rtc_peers.get(&peer_id))
    .map(|p| p.rtc_pending_answers.clone())
    .unwrap_or_default();

// ... remove old entry, insert new entry ...

let mut new_peer = torrent_peer;
if !old_rtc_pending_answers.is_empty() {
    new_peer.rtc_pending_answers = old_rtc_pending_answers;
}
entry.rtc_seeds.insert(peer_id, new_peer);
```

Failure to do this results in intermittent connection failures where leechers deposit answers that the seeder never receives.

---

## 8. WebRTC Data Channel Protocol

Once the WebRTC connection is established, peers communicate over a binary data channel using a simple custom protocol. No BitTorrent wire protocol (BEP-3) is used.

### 8.1 Channel Configuration

The seeder opens the data channel with:

```javascript
pc.createDataChannel('torrent', { ordered: false, maxRetransmits: 3 })
```

- **`ordered: false`** — Out-of-order delivery is acceptable; piece ordering is handled at the application layer.
- **`maxRetransmits: 3`** — Limits retransmission attempts; undelivered messages are dropped rather than blocking the channel. The application layer handles re-requests.
- **Channel name: `'torrent'`** — Informational only; clients should not depend on this string.

The leecher receives the channel via `ondatachannel`. It sets `channel.binaryType = 'arraybuffer'` before processing messages.

### 8.2 Message Types

All messages are binary `ArrayBuffer`s. The first byte is the message type identifier:

| Constant | Value | Direction | Description |
|----------|-------|-----------|-------------|
| `MSG_PIECE_REQUEST` | `0x01` | Leecher → Seeder | Request a piece by index |
| `MSG_PIECE_DATA` | `0x02` | Seeder → Leecher | Deliver a complete piece (≤ 65531 bytes) |
| `MSG_HAVE` | `0x03` | Either | Reserved / not currently used |
| `MSG_PIECE_CHUNK` | `0x04` | Seeder → Leecher | Deliver one chunk of a multi-chunk piece (> 65531 bytes) |

### 8.3 MSG_PIECE_REQUEST (0x01)

Sent by the leecher to request a specific piece.

```
Offset  Size  Type      Description
------  ----  --------  -----------
0       1     uint8     Message type = 0x01
1       4     uint32be  Piece index (big-endian)
```

Total size: **5 bytes**.

### 8.4 MSG_PIECE_DATA (0x02)

Sent by the seeder to deliver an entire piece when the piece fits within SCTP's maximum payload size (65,531 bytes).

```
Offset  Size      Type      Description
------  --------  --------  -----------
0       1         uint8     Message type = 0x02
1       4         uint32be  Piece index (big-endian)
5       variable  bytes     Raw piece data
```

Total size: **5 + piece_length bytes**.

### 8.5 MSG_PIECE_CHUNK (0x04)

Used when a piece exceeds 65,531 bytes. The seeder splits the piece into 16 KiB chunks and sends each as a separate `MSG_PIECE_CHUNK` message. The leecher reassembles them in a buffer keyed by piece index and processes the complete piece only when `received >= total`.

```
Offset  Size      Type      Description
------  --------  --------  -----------
0       1         uint8     Message type = 0x04
1       4         uint32be  Piece index (big-endian)
5       4         uint32be  Total piece size in bytes (big-endian)
9       4         uint32be  Byte offset of this chunk within the piece (big-endian)
13      variable  bytes     Chunk data (up to 16,384 bytes)
```

Total size: **13 + chunk_length bytes**.

### 8.6 Chunked Transfer for Large Pieces

The SCTP implementation underlying WebRTC DataChannel imposes a payload limit of approximately 65,531 bytes per message. When the piece size exceeds this limit, the seeder uses `MSG_PIECE_CHUNK` (0x04) for all chunks of that piece:

```
Chunk size = 16,384 bytes (16 KiB)
Threshold  = 65,531 bytes (SCTP_MAX_PAYLOAD)

if piece_length <= 65531:
    send MSG_PIECE_DATA (single message)
else:
    for offset in range(0, piece_length, 16384):
        send MSG_PIECE_CHUNK (header + chunk)
```

Reassembly on the leecher side:

```javascript
// Leecher maintains: _pieceChunks: Map<pieceIndex, { total, buf, received }>
_handlePieceChunk(pieceIndex, totalSize, offset, chunkData, peerInfo) {
    // Allocate buffer on first chunk
    if (!this._pieceChunks.has(pieceIndex)) {
        this._pieceChunks.set(pieceIndex, {
            total: totalSize, buf: new Uint8Array(totalSize), received: 0
        });
    }
    const entry = this._pieceChunks.get(pieceIndex);
    entry.buf.set(chunkData, offset);
    entry.received += chunkData.length;
    if (entry.received >= entry.total) {
        this._pieceChunks.delete(pieceIndex);
        this.handlePieceData(pieceIndex, entry.buf, peerInfo); // verify + store
    }
}
```

### 8.7 Flow Control

The data channel's send buffer can fill up if the seeder sends too fast. The implementation gates sends on `channel.bufferedAmount`:

- If `bufferedAmount > 512 KiB`, wait for `bufferedAmountLowThreshold = 64 KiB` before sending the next chunk.
- The seeder queues piece-serve operations serially per channel using a `Promise` chain (`channel._sendQueue`), so that multiple simultaneous requests do not interleave their chunks.

```javascript
handlePieceRequest(pieceIndex, channel) {
    if (!channel._sendQueue) channel._sendQueue = Promise.resolve();
    channel._sendQueue = channel._sendQueue.then(
        () => this._servePiece(pieceIndex, channel)
    );
}
```

---

## 9. Client-Side Implementation Guide

### 9.1 Peer Identity

Each `Torrent` instance generates a unique 20-byte `peer_id` at creation time using the standard BEP-3 convention of an ASCII prefix followed by random digits:

```
Prefix:   "-RT1000-"  (8 bytes)
Suffix:   12 random decimal digits
Total:    20 bytes
```

The `peer_id` is URL-percent-encoded byte-by-byte when included in the announce query string (every byte as `%XX`, even printable ASCII). This avoids ambiguity with URL special characters.

When a leecher targets a seeder in `rtcanswerfor`, it hex-encodes the 20-byte `peer_id` it received from the `rtc_peers` list (where `peer_id` is returned as raw binary). The resulting 40-character lowercase hex string is URL-encoded and appended to the announce.

### 9.2 ICE and SDP Lifecycle

**Seeder side:**

1. Create `RTCPeerConnection` with STUN servers.
2. Create data channel: `{ ordered: false, maxRetransmits: 3 }`.
3. Call `createOffer()` and `setLocalDescription()`.
4. Wait for ICE gathering to complete (state = `'complete'`) or timeout at 5 seconds — whichever comes first. Waiting ensures all ICE candidates are embedded in the SDP before it is published.
5. Cache `pc.localDescription.sdp` as `_localSdp`. Re-use this on subsequent announces until the connection is established or the `RTCPeerConnection` is closed.
6. After receiving an answer via `rtc_answers`: call `setRemoteDescription({ type: 'answer', sdp })`. Then clear `_localPc` and `_localSdp` so that the next announce cycle creates a fresh offer.

**Leecher side:**

1. Create `RTCPeerConnection` with STUN servers.
2. Register `ondatachannel` handler.
3. Call `setRemoteDescription({ type: 'offer', sdp: peerInfo.sdp_offer })`.
4. Call `createAnswer()` and `setLocalDescription()`.
5. Wait for ICE gathering to complete (same 5-second timeout).
6. Send `rtcanswer` + `rtcanswerfor` announce.

**SDP offer reuse:** The seeder reuses the same SDP offer across multiple announce cycles. It only creates a new offer when the connection is established (or when `signalingState` is no longer `'have-local-offer'`). This reduces STUN traffic and keeps the signaling flow efficient.

### 9.3 Announce Loop

The client runs a periodic announce loop that fires every `rtc interval` milliseconds (default: 10,000 ms, configurable by the tracker):

```
loop:
    if isSeeder:
        sdpOffer = getOrCreateSdpOffer()
        response = announce(rtctorrent=1, rtcoffer=sdpOffer)
        for each answer in response.rtc_answers:
            handleAnswerFromTracker(answer)
    else:
        response = announce(rtctorrent=1, rtcrequest=1)
        for each peer in response.rtc_peers:
            if peer.peer_id not in connected peers:
                connectToWebRTCPeer(peer)
        requestMissingPieces()

    schedule next announce at response.rtc_interval (or default)
```

Trackers that do not support RTC (return `failure reason` containing "rtctorrent", or omit `rtc_peers`/`rtc_answers` from a successful response) are added to a per-session ignore list. The client continues announcing to other trackers in the list.

### 9.4 In-Flight Request Management

The leecher maintains a `requestedPieces: Set<pieceIndex>` and a `MAX_IN_FLIGHT = 64` constant. On each announce or channel open event, it calls `_requestMissingPieces()`:

```javascript
_requestMissingPieces() {
    const toRequest = MAX_IN_FLIGHT - this.requestedPieces.size;
    for (let i = 0; i < this.pieceCount && requested < toRequest; i++) {
        if (!this.pieces.has(i) && !this.requestedPieces.has(i)) {
            this.requestPieceFromPeers(i);
            this.requestedPieces.add(i);
        }
    }
}
```

**Critical:** When a data channel opens (`channel.onopen`), `requestedPieces` must be **cleared before** calling `_requestMissingPieces()`. Without this, pieces that were requested but not yet received before the channel opened will remain in `requestedPieces` and will never be re-requested, causing a permanent stall for those pieces:

```javascript
channel.onopen = () => {
    this.requestedPieces.clear(); // REQUIRED
    this._requestMissingPieces();
};
```

Piece requests are **round-robined** across available peers: `openPeers[pieceIndex % openPeers.length]`. This distributes load across multiple seeders when available.

### 9.5 Piece Verification

Received pieces are verified against the torrent's piece hashes before being stored.

**BitTorrent v1:** SHA-1 hash of the piece data is compared against the 20-byte entry at `pieces[pieceIndex * 20 .. pieceIndex * 20 + 20]`.

**BitTorrent v2:** Not implemented in the piece-exchange layer (the `pieces` field may be absent for pure v2 torrents). All pieces are accepted if no hash is present (the torrent can be used with trusted seeders).

**Hash mismatch:** The piece is discarded and removed from `requestedPieces`, allowing it to be re-requested on the next `_requestMissingPieces()` call. No peer is immediately penalized for a single hash mismatch.

### 9.6 Peer Speed Monitoring and Blacklisting

The leecher tracks response latency per peer:

- Each pending piece request is timestamped in `_peerStats.get(peerId).pendingRequests`.
- On successful receipt, the round-trip time is appended to `responseTimes` (capped at 10 entries).
- A sliding-window average over the last 5 entries is computed every 10 seconds.
- If the average exceeds `slowPeerThresholdMs` (default: 8,000 ms), the peer is blacklisted.
- If 3 or more requests time out within `peerRequestTimeoutMs` (default: 30,000 ms), the peer is blacklisted.

Blacklisted peers are excluded from future piece requests. Their in-flight pieces are removed from `requestedPieces` and re-requested from other peers.

### 9.7 WebSeed Fallback (BEP-19)

If no open data channel is available when a piece is requested, and the torrent has `url-list` entries (WebSeed URLs), the leecher fetches the piece via HTTP Range request:

```
GET <webseed_url> HTTP/1.1
Range: bytes=<piece_start>-<piece_end>
```

For multi-file torrents, the piece may span files; the leecher issues one range request per overlapping file and assembles the piece from the responses. The piece is verified with the same SHA-1 hash check before storage.

---

## 10. Torrent Format Support

### 10.1 BitTorrent v1 (SHA-1)

Standard BEP-3 torrent format. The `info` dictionary contains:
- `name`: Torrent name
- `piece length`: Bytes per piece (dynamic based on total size — see below)
- `pieces`: Concatenated 20-byte SHA-1 hashes of all pieces
- `length` (single-file) or `files` (multi-file): File layout

**Dynamic piece length selection:**

| Total File Size | Piece Length |
|-----------------|--------------|
| ≤ 8 MB | 16 KiB |
| 8 MB – 64 MB | 32 KiB |
| > 64 MB | 64 KiB |

The torrent info-hash is SHA-1 of the bencoded `info` dictionary. Magnet URI format: `magnet:?xt=urn:btih:<40-char-hex>`.

### 10.2 BitTorrent v2 (BEP-52, SHA-256 Merkle)

BEP-52 defines a Merkle-tree-based file hashing scheme using SHA-256:

- Each file is divided into 16 KiB blocks.
- Blocks are SHA-256 hashed to form leaf nodes.
- The tree is padded to the next power of two with zero-hashes.
- Interior nodes are the SHA-256 of their two children concatenated.
- The Merkle root of each file is stored in the `file tree` dictionary.

The `info` dictionary contains:
- `file tree`: Nested dictionary mapping file paths to their Merkle root hashes
- `meta version`: Integer `2`
- `piece length`: Must be a power of two ≥ 16 KiB
- No `pieces` field

For files larger than one piece, the hashes at the appropriate tree level (`piece layers`) are stored in the top-level `piece layers` dictionary of the `.torrent` file.

The info-hash for v2 is the SHA-256 of the bencoded `info` dictionary. Magnet URI format: `magnet:?xt=urn:btmh:1220<64-char-hex>`.

### 10.3 Hybrid Torrents

Hybrid torrents are backward-compatible with v1 while also supporting v2 hash trees. The `info` dictionary contains all v1 fields (`pieces`, `length`/`files`) and all v2 fields (`file tree`, `meta version: 2`). Clients that support only v1 ignore the v2 fields; v2 clients use the Merkle hashes.

The torrent has two info-hashes: a v1 SHA-1 hash and a v2 SHA-256 hash. The magnet URI includes both:
```
magnet:?xt=urn:btih:<v1_hex>&xt=urn:btmh:1220<v2_hex>&dn=...&tr=...
```

Parsing logic: if either `file tree` or `meta version: 2` is present in the `info` dict, the torrent is treated as v2/hybrid.

---

## 11. Tracker Implementation Guide

This section describes the minimal set of changes needed to add RtcTorrent support to an existing HTTP BitTorrent tracker.

### 11.1 Parsing Announce Requests

Add parsing for the five new parameters. All are optional; if absent, the announce is treated as a standard (non-RTC) announce:

```
rtctorrent  := query["rtctorrent"] == "1"  → bool
rtcoffer    := query["rtcoffer"]           → Option<String>  (URL-decoded by HTTP layer)
rtcrequest  := query["rtcrequest"] == "1"  → bool
rtcanswer   := query["rtcanswer"]          → Option<String>  (URL-decoded by HTTP layer)
rtcanswerfor := query["rtcanswerfor"]      → Option<String>  (40-char hex, URL-decoded)
```

Most HTTP frameworks automatically URL-decode query parameter values. Verify this, because SDP strings contain characters that require encoding (`=`, `+`, `/`, spaces rendered as `%20`).

### 11.2 Storing Peers

On announce, create or update the peer record with the RTC fields:

1. Set `is_rtctorrent = true` if `rtctorrent == 1`.
2. Set `rtc_sdp_offer = rtcoffer` if present; otherwise leave existing value.
3. Route the peer: if `left == 0`, insert into `rtc_seeds`; otherwise into `rtc_peers`.
4. **Preserve `rtc_pending_answers`** when updating an existing peer record (see §7.3).

If `rtcanswer` and `rtcanswerfor` are both present:
1. Hex-decode `rtcanswerfor` to get the 20-byte seeder `peer_id`.
2. Look up the target peer in `rtc_seeds` or `rtc_peers` for this `info_hash`.
3. Append `(current_peer_id, rtcanswer)` to that peer's `rtc_pending_answers`.

### 11.3 Building the RTC Response

When `rtctorrent == 1`, return the RTC-specific bencoded response instead of the normal peers list:

```
response = {
    "rtc interval": <config.rtc_interval as integer milliseconds>,
    "complete":     <rtc_seeds.len()>,
    "incomplete":   <rtc_peers.len()>,
    "rtc_peers":    [ for each peer in rtc_seeds where peer.rtc_sdp_offer is not empty:
                        { "peer_id": <20-byte binary>, "sdp_offer": <string> }
                    ],
    "rtc_answers":  [ for each (answerer_id, sdp) in atomic_take(peer.rtc_pending_answers):
                        { "peer_id": <20-byte binary>, "sdp_answer": <string> }
                    ]
}
```

**Exclusion rule:** Do not include the requesting peer's own `peer_id` in `rtc_peers`. A seeder should not see itself in the list; a leecher should not try to connect to itself.

**Seeder-specific rule:** Leechers should only see `rtc_seeds` entries in `rtc_peers` (seeders with offers), not other leechers. Leechers do not publish SDP offers.

### 11.4 Answer Queue Atomicity

The `take_rtc_pending_answers` operation must be **atomic** (write-locked):

```rust
// Atomically drain the pending answers for a peer
pub fn take_rtc_pending_answers(&self, info_hash: InfoHash, peer_id: PeerId)
    -> Vec<(PeerId, String)>
{
    // acquire write lock on the torrent shard
    let mut lock = shard.write();
    if let Some(torrent_entry) = lock.get_mut(&info_hash) {
        if let Some(p) = torrent_entry.rtc_seeds.get_mut(&peer_id)
            .or_else(|| torrent_entry.rtc_peers.get_mut(&peer_id))
        {
            return std::mem::take(&mut p.rtc_pending_answers);
        }
    }
    Vec::new()
}
```

Using `std::mem::take` (or equivalent) rather than `clone()` ensures the queue is cleared in the same operation that reads it, preventing double-delivery.

### 11.5 Filtering Peers

`get_rtctorrent_peers` implements the view returned to each requester:

| Requester Role | `rtc_seeds` in response | `rtc_peers` in response |
|----------------|------------------------|------------------------|
| Seeder (`left == 0`) | All except self | All except self |
| Leecher (`left > 0`) | All except self | **Empty** (cleared) |

Leechers only receive seeder offers, not other leechers' entries. This is consistent with standard BitTorrent behavior where leechers predominantly connect to seeders.

### 11.6 Configuration

Two tracker configuration values affect RtcTorrent behavior:

| Config Key | Type | Default | Description |
|------------|------|---------|-------------|
| `rtc_interval` | integer (ms) | `10000` | How often clients should re-announce. Returned in all responses (RTC and non-RTC) as `"rtc interval"`. |
| `rtc_peers_timeout` | integer (s) | — | Peer expiry timeout for RTC peers (same eviction logic as non-RTC peers). |
| `rtctorrent` (per HTTP tracker) | bool | `false` | Enable/disable RTC signaling support per tracker instance. Returns `failure reason: rtctorrent not enabled` when disabled. |

---

## 12. URL Encoding Requirements

SDP strings contain characters with special meaning in URLs: `=`, `+`, `/`, `\r\n` (CRLF), spaces, etc. Proper encoding is essential.

**Correct approach:** Build the announce query string by manually concatenating parts, using `encodeURIComponent()` only on the SDP/answer values:

```javascript
parts.push('rtcoffer=' + encodeURIComponent(sdpOffer));
parts.push('rtcanswer=' + encodeURIComponent(answerSdp));
parts.push('rtcanswerfor=' + encodeURIComponent(peerIdHex));
const url = baseUrl + '?' + parts.join('&');
```

**Incorrect approach — double encoding:** Using `URLSearchParams.set()` followed by `toString()` will double-encode pre-encoded values. If you pass an already-encoded SDP to `URLSearchParams.set()`, the `%` characters will be re-encoded as `%25`, resulting in `%2520`, `%253D`, etc. The tracker will then receive a garbled SDP that fails WebRTC parsing.

For `info_hash` and `peer_id`, which are raw binary, percent-encode every byte individually:
```javascript
function urlEncodeBytes(bytes) {
    let out = '';
    for (let i = 0; i < bytes.length; i++) {
        out += '%' + bytes[i].toString(16).padStart(2, '0');
    }
    return out;
}
```

---

## 13. Browser Compatibility and CORS

Because browser-side JavaScript initiates announces from a different origin than the tracker, the tracker must respond with CORS headers permitting the request:

```
Access-Control-Allow-Origin: *
Access-Control-Allow-Headers: *
Access-Control-Max-Age: 3600
```

In Actix-Web (Rust), this is configured as:

```rust
Cors::default()
    .allow_any_origin()
    .send_wildcard()
    .allowed_methods(vec!["GET", "OPTIONS"])
    .allow_any_header()
    .max_age(3600)
```

HTTP tracker announce requests use `GET`, so only `GET` and `OPTIONS` (preflight) need to be allowed.

**HTTPS requirement:** Modern browsers require that WebRTC peer connections initiated from an HTTPS page also access HTTPS resources (mixed-content policy). If the web app is served over HTTPS, the tracker announce endpoint must also be HTTPS. The tracker supports TLS via rustls with dynamically loaded certificates.

**Native module loading in Node.js + Webpack:** The `@roamhq/wrtc` package (native Node.js WebRTC) uses a `.node` binary that cannot be loaded through webpack's module system. Use `__non_webpack_require__` (webpack's escape hatch for native requires) instead of `require`:

```javascript
const nativeRequire = (typeof __non_webpack_require__ !== 'undefined')
    ? __non_webpack_require__
    : require;
const { RTCPeerConnection } = nativeRequire('@roamhq/wrtc');
```

---

## 14. Known Pitfalls and Critical Implementation Notes

The following are implementation errors discovered and fixed in the reference implementation. Other implementors should be aware of these hazards.

### 14.1 Answer Queue Erasure on Re-Announce

**Symptom:** WebRTC connections intermittently fail. Leechers send answers but seeders never receive them.

**Cause:** Re-announce calls `add_torrent_peer`, which removes the old peer entry and inserts a new one. The new entry is initialized with `rtc_pending_answers: Vec::new()`, discarding any answers deposited between the previous poll and this re-announce.

**Fix:** Capture `old_rtc_pending_answers` from the existing entry before removal and restore it into the new entry after insertion (§7.3).

### 14.2 Double URL Encoding of SDP

**Symptom:** Tracker receives garbled SDP strings (e.g., `v%3D0` becomes `v%253D0`).

**Cause:** Using `URLSearchParams.set()` on a value that is already percent-encoded causes the `%` character to be re-encoded as `%25`.

**Fix:** Append RTC parameters by string concatenation with explicit `encodeURIComponent()` on the raw SDP value (§12).

### 14.3 requestedPieces Not Cleared on Channel Open

**Symptom:** Some pieces are never downloaded; download stalls at a percentage.

**Cause:** `_requestMissingPieces()` may run before the channel is open (e.g., triggered by the announce loop). Pieces are added to `requestedPieces` even though the send fails. When `onopen` fires and calls `_requestMissingPieces()`, those pieces are already in the set and are skipped.

**Fix:** Call `this.requestedPieces.clear()` in `channel.onopen` before calling `_requestMissingPieces()` (§9.4).

### 14.4 Incorrect `peer_id` Hex Encoding Length

**Symptom:** Tracker fails to route the answer to the correct seeder; `rtcanswerfor` decoding fails.

**Cause:** The `peer_id` in the `rtc_peers` response is a 20-byte binary value. When hex-encoded, it must be exactly 40 characters. Using `toString('hex')` on a Node.js `Buffer` produces the correct result; incorrectly treating it as a UTF-8 string and using `.charCodeAt()` or similar can produce wrong output.

**Fix:** Always use `Buffer.from(peer_id_bytes).toString('hex')` or equivalent to produce the 40-character hex string.

### 14.5 SDP Answer Applied to Wrong Signaling State

**Symptom:** `setRemoteDescription` throws `"InvalidStateError: Cannot set remote description in state stable"`.

**Cause:** The seeder tries to apply an SDP answer, but `signalingState` is no longer `'have-local-offer'` (e.g., the `RTCPeerConnection` was reused or closed).

**Fix:** Check `this._localPc.signalingState === 'have-local-offer'` before calling `setRemoteDescription`. If the state is wrong, discard the answer and wait for the next announce cycle to create a fresh offer.

---

## 15. Interoperability with Standard BitTorrent

RtcTorrent is designed to be fully additive and non-breaking:

- **Non-RTC clients** announcing to an RTC-enabled tracker receive the standard bencoded response. The only addition is `"rtc interval"` in the response, which non-RTC clients ignore.
- **RTC clients** announcing to a non-RTC tracker receive either a `failure reason` (which they detect and ignore for that tracker) or a response lacking `rtc_peers`/`rtc_answers` (which they also treat as non-RTC).
- **Mixed swarms** work: a torrent can have both RTC and non-RTC peers. RTC peers (browser/Node clients) connect to each other via WebRTC, while non-RTC peers continue using TCP connections. The tracker tracks both populations separately.
- **Existing BEPs respected:** The protocol builds on BEP-3 (base announce), BEP-7/23 (compact peer encoding), BEP-19 (WebSeed fallback), and BEP-52 (v2 torrent format). No existing BEPs are modified.

---

## 16. Security Considerations

**Tracker as passive relay:** The tracker does not validate SDP content, peer identities, or the accuracy of `left` values. It is a passive relay for opaque strings. This is the same trust model as standard BitTorrent trackers.

**SDP injection:** A malicious peer could provide a crafted SDP offer/answer. The receiving peer processes it only through the WebRTC API, which performs its own validation. The worst realistic outcome is a failed connection.

**Peer impersonation via `rtcanswerfor`:** A malicious peer could submit an answer claiming to be directed at any seeder's `peer_id`. The seeder will receive the answer and attempt to apply it as an SDP answer. If the answer is invalid SDP, `setRemoteDescription` will throw and the connection will fail gracefully. This cannot be prevented without authenticated signaling.

**STUN/TURN relay leaks:** If STUN/TURN servers are used, ICE candidates reveal the peer's public IP address to the tracker and to the other peer, as in any WebRTC deployment.

**Answer queue flooding:** A malicious peer could flood a seeder's `rtc_pending_answers` by submitting many fake answers. Trackers should consider rate-limiting answer submissions per `info_hash` per source IP, or imposing a maximum queue size per peer.

**CORS permissiveness:** The `Access-Control-Allow-Origin: *` header is required for browser clients but allows any web page to send announces. This is inherent to the browser-based torrent use case.

---

## 17. Reference Implementation Summary

The reference implementation consists of:

| Component | Language | Location | Purpose |
|-----------|----------|----------|---------|
| HTTP tracker with RTC signaling | Rust (Actix-Web) | `src/` | Tracker backend |
| RtcTorrent JS library | JavaScript (ES2020) | `lib/rtctorrent/src/rtctorrent.js` | Browser + Node.js client |
| CLI seeder | Node.js | `lib/rtctorrent/bin/seed.js` | File seeding from command line |
| Browser demo | HTML/JS | `lib/rtctorrent/demo/index.html` | Playback demo |
| Signaling test suite | Node.js | `lib/rtctorrent/test/test_signaling_flow.js` | 13 protocol assertions |
| Transfer test | Node.js | `lib/rtctorrent/test/test_webrtc_transfer.js` | End-to-end 5600-byte transfer |

**Key Rust source files:**

| File | Description |
|------|-------------|
| `src/tracker/structs/announce_query_request.rs` | Extended announce struct with RTC fields |
| `src/tracker/structs/torrent_peer.rs` | Extended peer struct with RTC fields |
| `src/tracker/impls/torrent_tracker_handlers.rs` | Announce parsing and routing |
| `src/tracker/impls/torrent_tracker_rtctorrent.rs` | RTC-specific tracker methods |
| `src/tracker/impls/torrent_tracker_peers.rs` | Peer add/remove with answer preservation |
| `src/http/http.rs` | HTTP response generation including RTC response |

**Default tracker endpoints:**
- HTTP tracker: `http://127.0.0.1:6969/announce`
- API: `http://127.0.0.1:8081`

---

## 18. Glossary

| Term | Definition |
|------|------------|
| **SDP** | Session Description Protocol. A text format describing the capabilities and network addresses of a WebRTC peer, used in offer/answer negotiation. |
| **ICE** | Interactive Connectivity Establishment. A framework for discovering usable network paths between peers, using STUN and optionally TURN servers. |
| **STUN** | Session Traversal Utilities for NAT. A protocol that allows peers to discover their public IP and port through a lightweight server. |
| **TURN** | Traversal Using Relays around NAT. A STUN extension that relays data through a server when direct connectivity is not possible. |
| **SDP Offer** | The initiator's (seeder's) WebRTC connection proposal, including codec capabilities and ICE candidates. |
| **SDP Answer** | The responder's (leecher's) acceptance and counter-parameters for the WebRTC connection. |
| **DataChannel** | A WebRTC channel for arbitrary binary or text data, not audio/video. Used here for piece transfer. |
| **Piece** | A fixed-size block of torrent data defined by `piece length` in the torrent info dictionary. |
| **SCTP** | Stream Control Transmission Protocol. The transport layer underlying WebRTC DataChannel. |
| **BEP** | BitTorrent Enhancement Proposal. The standardization mechanism for BitTorrent protocol extensions. |
| **info-hash** | The SHA-1 (v1) or SHA-256 (v2) hash of the bencoded `info` dictionary. Uniquely identifies a torrent. |
| **Seeder** | A peer with `left=0`, holding a complete copy of the torrent and serving pieces. |
| **Leecher** | A peer with `left > 0`, still downloading the torrent. |
| **rtc_seeds** | Tracker-side map of RTC-capable seeders (peers with `left=0` and `rtctorrent=1`). |
| **rtc_peers** | Tracker-side map of RTC-capable leechers (peers with `left>0` and `rtctorrent=1`). |
| **Pending answers** | A queue on the tracker attached to each seeder peer record, holding SDP answers from leechers not yet collected by the seeder. |
| **Hybrid torrent** | A `.torrent` file containing both v1 (SHA-1) and v2 (SHA-256 Merkle) hashing metadata for backward compatibility. |
