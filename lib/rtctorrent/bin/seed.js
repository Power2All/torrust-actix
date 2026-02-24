#!/usr/bin/env node
/**
 * RtcTorrent CLI seeder
 *
 * Usage:
 *   node seed.js [--tracker <url>] [--webseed <url>] <file1> [<file2> ...]
 *
 * Options:
 *   --tracker  Tracker announce URL (default: http://127.0.0.1:6969/announce)
 *   --name     Torrent name (default: first file's basename)
 *   --out      Save .torrent file to this path (default: <name>.torrent)
 *   --webseed  HTTP URL for BEP-19 web seeding fallback (can be repeated for multiple URLs)
 *
 * Example:
 *   node seed.js --tracker http://mytracker:6969/announce /videos/movie.mp4
 *
 * Output:
 *   Prints the magnet URI and saves a .torrent file you can share with leechers.
 *   The browser demo page (demo/index.html) can open either.
 */
'use strict';

const path = require('path');
const fs   = require('fs');

const args = process.argv.slice(2);
let trackerUrl = 'http://127.0.0.1:6969/announce';
let torrentName = null;
let outFile = null;
const filePaths = [];
const webseedUrls = [];
for (let i = 0; i < args.length; i++) {
    if (args[i] === '--tracker' && args[i + 1]) { trackerUrl   = args[++i]; }
    else if (args[i] === '--name'    && args[i + 1]) { torrentName = args[++i]; }
    else if (args[i] === '--out'     && args[i + 1]) { outFile     = args[++i]; }
    else if (args[i] === '--webseed' && args[i + 1]) { webseedUrls.push(args[++i]); }
    else if (!args[i].startsWith('--')) { filePaths.push(args[i]); }
}
if (filePaths.length === 0) {
    console.error('Usage: node seed.js [--tracker <url>] [--name <name>] [--webseed <url>] <file1> [<file2> ...]');
    process.exit(1);
}
for (const p of filePaths) {
    if (!fs.existsSync(p)) {
        console.error(`File not found: ${p}`);
        process.exit(1);
    }
}
torrentName = torrentName || path.basename(filePaths[0], path.extname(filePaths[0]));
outFile = outFile || `${torrentName}.torrent`;
const distPath = path.join(__dirname, '..', 'dist', 'rtctorrent.node.js');
if (!fs.existsSync(distPath)) {
    console.error(`Built library not found at ${distPath}`);
    console.error('Run:  npm run build  inside lib/rtctorrent/');
    process.exit(1);
}
const lib = require(distPath);
const RtcTorrent = lib.default || lib.RtcTorrent || lib;

async function main() {
    console.log('=== RtcTorrent Seeder ===');
    console.log(`Tracker : ${trackerUrl}`);
    console.log(`Files   : ${filePaths.join(', ')}`);
    if (webseedUrls.length) {
        console.log(`Webseeds: ${webseedUrls.join(', ')}`);
    }
    console.log('');
    const client = new RtcTorrent({
        trackerUrl,
        rtcInterval: 5000,
        iceServers: [
            { urls: 'stun:stun.l.google.com:19302' },
            { urls: 'stun:stun1.l.google.com:19302' }
        ]
    });
    process.stdout.write('Creating torrent (hashing pieces)… ');
    let result;
    try {
        result = await client.create(filePaths, {
            name: torrentName,
            webseedUrls: webseedUrls.length ? webseedUrls : undefined,
        });
    } catch (e) {
        console.error('\nFailed to create torrent:', e.message);
        process.exit(1);
    }
    const { infoHash, encodedTorrent, magnetUri } = result;
    console.log('done.');
    fs.writeFileSync(outFile, Buffer.from(encodedTorrent));
    console.log(`\nSaved : ${outFile}`);
    console.log(`Hash  : ${infoHash}`);
    console.log(`\nMagnet URI:\n${magnetUri}\n`);
    console.log('Share the magnet URI or the .torrent file with leechers.\n');
    await client.seed(encodedTorrent, filePaths);
    console.log('Seeding… (Ctrl+C to stop)\n');
    setInterval(() => {
        const torrent = [...client.torrents.values()][0];
        if (!torrent) return;
        console.log(
            `[${new Date().toLocaleTimeString()}] peers: ${torrent.peers.size}` +
            `  uploaded: ${fmt(torrent.uploaded)}`
        );
    }, 10000);
}

function fmt(bytes) {
    if (bytes < 1024)        return bytes + ' B';
    if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB';
    if (bytes < 1024 ** 3)   return (bytes / 1024 / 1024).toFixed(1) + ' MB';
    return (bytes / 1024 / 1024 / 1024).toFixed(2) + ' GB';
}

main().catch(e => { console.error('Fatal:', e); process.exit(1); });