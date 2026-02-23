/**
 * End-to-end WebRTC file transfer test.
 *
 * Tests the complete flow:
 *   1. Seeder creates a torrent, generates a real SDP offer (via @roamhq/wrtc)
 *   2. Tracker stores the offer
 *   3. Leecher gets the offer, creates a real SDP answer
 *   4. Tracker delivers the answer to the seeder
 *   5. WebRTC data channel opens on both sides
 *   6. Leecher requests pieces → seeder sends → leecher reassembles
 *   7. Verify the received data matches the original
 *
 * Run: node test_webrtc_transfer.js
 * Requires: tracker on http://127.0.0.1:6969, @roamhq/wrtc installed
 */

'use strict';

const path = require('path');
const fs = require('fs');
const os = require('os');
const builtLib = require('../dist/rtctorrent.node.js');
const RtcTorrent = builtLib.default || builtLib.RtcTorrent || builtLib;
const TRACKER_URL = 'http://127.0.0.1:6969/announce';
const RTC_INTERVAL = 3000;
const TEST_TIMEOUT = 60000;

function sleep(ms) { return new Promise(r => setTimeout(r, ms)); }

async function main() {
    console.log('=== WebRTC Transfer Test ===\n');
    const testContent = Buffer.from(
        'Hello WebRTC! '.repeat(400)
    );
    const testFile = path.join(os.tmpdir(), `rtctest_${Date.now()}.bin`);
    fs.writeFileSync(testFile, testContent);
    console.log(`Test file: ${testFile}  (${testContent.length} bytes)`);
    const seederClient = new RtcTorrent({
        trackerUrl: TRACKER_URL,
        rtcInterval: RTC_INTERVAL,
        iceServers: []
    });
    console.log('\n--- Creating torrent ---');
    let torrentResult;
    try {
        torrentResult = await seederClient.create([testFile], { name: 'rtctest' });
    } catch (e) {
        console.error('create() failed:', e.message);
        process.exit(1);
    }
    const { infoHash, encodedTorrent } = torrentResult;
    console.log(`Info hash: ${infoHash}`);
    console.log('\n--- Starting seeder ---');
    let seederTorrent;
    try {
        seederTorrent = await seederClient.seed(encodedTorrent, [testFile]);
    } catch (e) {
        console.error('seed() failed:', e.message);
        process.exit(1);
    }
    console.log('Seeder started. Waiting for ICE gathering + first announce...');
    await sleep(8000);
    console.log('Seeder announce done.');
    console.log('\n--- Starting leecher ---');
    const leecherClient = new RtcTorrent({
        trackerUrl:  TRACKER_URL,
        rtcInterval: RTC_INTERVAL,
        iceServers:  []
    });
    let downloadComplete = false;
    let leecherTorrent;
    try {
        leecherTorrent = await leecherClient.download(encodedTorrent);
    } catch (e) {
        console.error('download() failed:', e.message);
        cleanup(testFile, seederTorrent, leecherTorrent);
        process.exit(1);
    }
    leecherTorrent.onDownloadComplete = function () {
        console.log('\n[LEECHER] Download complete!');
        downloadComplete = true;
    };
    console.log('Leecher started. Waiting for WebRTC connection + transfer...\n');
    const deadline = Date.now() + TEST_TIMEOUT;
    let lastPct = -1;
    while (!downloadComplete && Date.now() < deadline) {
        await sleep(1000);
        const total = leecherTorrent.totalSize || 1;
        const dl = leecherTorrent.downloaded;
        const pct = Math.floor(dl / total * 100);
        if (pct !== lastPct) {
            console.log(`  Progress: ${dl}/${total} bytes  (${pct}%)`);
            lastPct = pct;
        }
        const peers = leecherTorrent.peers;
        if (peers.size > 0) {
            for (const [pid, peer] of peers) {
                const cs = peer.connection?.connectionState || '?';
                const dc = peer.channel?.readyState || 'no channel';
                console.log(`  Peer ${pid.slice(0, 8)}: conn=${cs}  channel=${dc}`);
            }
        }
    }
    console.log('\n=== Verification ===');
    if (!downloadComplete) {
        console.error('FAIL ✗ — download timed out');
        console.log('Seeder peers:', seederTorrent.peers.size);
        console.log('Leecher peers:', leecherTorrent.peers.size);
        for (const [pid, peer] of leecherTorrent.peers) {
            console.log(`  peer ${pid}: conn=${peer.connection?.connectionState}, dc=${peer.channel?.readyState}`);
        }
        cleanup(testFile, seederTorrent, leecherTorrent);
        process.exit(1);
    }
    const pieceCount = leecherTorrent.pieceCount;
    const pieces = [];
    for (let i = 0; i < pieceCount; i++) {
        if (leecherTorrent.pieces.has(i)) {
            pieces.push(Buffer.from(leecherTorrent.pieces.get(i)));
        } else {
            console.error(`FAIL ✗ — piece ${i} missing`);
            cleanup(testFile, seederTorrent, leecherTorrent);
            process.exit(1);
        }
    }
    const received = Buffer.concat(pieces).slice(0, testContent.length);
    const matches = received.equals(testContent);
    console.log(`Original : ${testContent.length} bytes`);
    console.log(`Received : ${received.length} bytes`);
    console.log(`Data match: ${matches ? 'PASS ✓' : 'FAIL ✗'}`);
    if (!matches) {
        for (let i = 0; i < Math.min(received.length, testContent.length); i++) {
            if (received[i] !== testContent[i]) {
                console.error(`First mismatch at byte ${i}`);
                break;
            }
        }
    }
    cleanup(testFile, seederTorrent, leecherTorrent);
    process.exit(matches ? 0 : 1);
}

function cleanup(testFile, seederTorrent, leecherTorrent) {
    try { seederTorrent?.stop(); } catch (_) {}
    try { leecherTorrent?.stop(); } catch (_) {}
    try { fs.unlinkSync(testFile); } catch (_) {}
}

main().catch(err => {
    console.error('Unhandled error:', err);
    process.exit(2);
});