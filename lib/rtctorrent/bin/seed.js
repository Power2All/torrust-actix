#!/usr/bin/env node
/**
 * RtcTorrent CLI seeder
 *
 * Single-torrent mode:
 *   node seed.js [--tracker <url>] [--name <name>] [--out <path>]
 *                [--webseed <url>] ... <file1> [<file2> ...]
 *
 * Multi-torrent mode (YAML):
 *   node seed.js --torrents <config.yaml>
 *
 * YAML format:
 *   ---
 *   torrents:
 *     - out: "/path/to/output.torrent"      # optional
 *       name: "My Movie"                     # optional
 *       file:
 *         - "/path/to/movie.mp4"
 *       trackers:
 *         - "http://localhost:6969/announce"
 *       webseed:                             # optional
 *         - "https://cdn.example.com/movie.mp4"
 *       ice:                                 # optional
 *         - "stun:stun.l.google.com:19302"
 *       rtc_interval: 10000                  # optional, milliseconds
 *
 * Reload (multi-torrent mode only):
 *   - Linux/macOS: send SIGHUP  →  kill -HUP <pid>
 *   - Windows / all platforms:  edit and save the YAML file (polled every 2 s)
 *   On reload the process exits with code 0; a process manager (pm2, systemd)
 *   should restart it to pick up the new config.
 */
'use strict';

const path = require('path');
const fs   = require('fs');

// ─── Argument parsing ────────────────────────────────────────────────────────

const args = process.argv.slice(2);

let torrentsFile    = null;   // --torrents
let trackerUrl      = 'http://127.0.0.1:6969/announce';
let torrentName     = null;
let outFile         = null;
let torrentVersion  = 'v1';   // --torrent-version: v1 | v2 | hybrid
const filePaths     = [];
const webseedUrls   = [];

for (let i = 0; i < args.length; i++) {
    if      (args[i] === '--torrents'        && args[i + 1]) { torrentsFile   = args[++i]; }
    else if (args[i] === '--tracker'         && args[i + 1]) { trackerUrl     = args[++i]; }
    else if (args[i] === '--name'            && args[i + 1]) { torrentName    = args[++i]; }
    else if (args[i] === '--out'             && args[i + 1]) { outFile        = args[++i]; }
    else if (args[i] === '--webseed'         && args[i + 1]) { webseedUrls.push(args[++i]); }
    else if (args[i] === '--torrent-version' && args[i + 1]) { torrentVersion = args[++i]; }
    else if (!args[i].startsWith('--'))                      { filePaths.push(args[i]); }
}

// ─── Mode dispatch ───────────────────────────────────────────────────────────

if (torrentsFile) {
    // Validate: no single-torrent flags allowed alongside --torrents
    if (filePaths.length > 0 || torrentName || outFile || webseedUrls.length > 0) {
        console.error(
            'Error: --torrents cannot be combined with single-torrent options ' +
            '(positional files, --name, --out, --webseed).'
        );
        process.exit(1);
    }
    if (!fs.existsSync(torrentsFile)) {
        console.error(`Torrents file not found: ${torrentsFile}`);
        process.exit(1);
    }
    runTorrentsMode(torrentsFile).catch(e => { console.error('Fatal:', e); process.exit(1); });
} else {
    if (filePaths.length === 0) {
        console.error(
            'Usage: node seed.js [--tracker <url>] [--name <name>] [--webseed <url>] <file1> ...\n' +
            '       node seed.js --torrents <config.yaml>'
        );
        process.exit(1);
    }
    for (const p of filePaths) {
        if (!fs.existsSync(p)) { console.error(`File not found: ${p}`); process.exit(1); }
    }
    main().catch(e => { console.error('Fatal:', e); process.exit(1); });
}

// ─── Library bootstrap ───────────────────────────────────────────────────────

function loadLib() {
    const distPath = path.join(__dirname, '..', 'dist', 'rtctorrent.node.js');
    if (!fs.existsSync(distPath)) {
        console.error(`Built library not found at ${distPath}`);
        console.error('Run:  npm run build  inside lib/rtctorrent/');
        process.exit(1);
    }
    const lib = require(distPath);
    return lib.default || lib.RtcTorrent || lib;
}

// ─── Single-torrent mode ─────────────────────────────────────────────────────

async function main() {
    const RtcTorrent = loadLib();

    if (filePaths.length === 1 && !torrentName) {
        torrentName = path.basename(filePaths[0]);
        outFile = outFile || path.basename(filePaths[0], path.extname(filePaths[0])) + '.torrent';
    } else {
        torrentName = torrentName || path.basename(filePaths[0], path.extname(filePaths[0]));
        outFile = outFile || `${torrentName}.torrent`;
    }

    console.log('=== RtcTorrent Seeder ===');
    console.log(`Tracker : ${trackerUrl}`);
    console.log(`Files   : ${filePaths.join(', ')}`);
    if (webseedUrls.length) console.log(`Webseeds: ${webseedUrls.join(', ')}`);
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
            version: torrentVersion,
            webseedUrls: webseedUrls.length ? webseedUrls : undefined,
        });
    } catch (e) {
        console.error('\nFailed to create torrent:', e.message);
        process.exit(1);
    }

    const { infoHash, encodedTorrent, magnetUri, v2InfoHash } = result;
    console.log('done.');
    fs.writeFileSync(outFile, Buffer.isBuffer(encodedTorrent) ? encodedTorrent : Buffer.from(encodedTorrent));
    console.log(`\nSaved : ${outFile}`);
    console.log(`Hash  : ${infoHash}`);
    if (v2InfoHash) console.log(`v2Hash: ${v2InfoHash}`);
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

// ─── Multi-torrent mode ───────────────────────────────────────────────────────

/** Load and parse the YAML config file. */
function loadYaml(filePath) {
    // js-yaml is a regular dependency — load it here (not at module top) so that
    // single-torrent mode doesn't require it to be installed.
    let yaml;
    try {
        yaml = require('js-yaml');
    } catch (_) {
        console.error('Missing dependency: js-yaml.  Run:  npm install  inside lib/rtctorrent/');
        process.exit(1);
    }
    const content = fs.readFileSync(filePath, 'utf8');
    return yaml.load(content);
}

/** Seed a single entry from the YAML config. */
async function seedEntry(entry) {
    const RtcTorrent = loadLib();

    const entryTrackers = entry.trackers || [];
    if (entryTrackers.length === 0) {
        throw new Error('torrent entry has no trackers');
    }
    const entryFiles = entry.file || [];
    if (entryFiles.length === 0) {
        throw new Error('torrent entry has no files');
    }

    for (const p of entryFiles) {
        if (!fs.existsSync(p)) throw new Error(`File not found: ${p}`);
    }

    const entryTracker   = entryTrackers[0];
    const entryWebseeds  = entry.webseed || [];
    const entryIce       = (entry.ice || []).map(u => ({ urls: u }));
    const entryInterval  = entry.rtc_interval || 5000;
    const entryName = entry.name ||
        (entryFiles.length === 1
            ? path.basename(entryFiles[0])
            : path.basename(entryFiles[0], path.extname(entryFiles[0])));
    const entryOut = entry.out ||
        (entryFiles.length === 1
            ? path.basename(entryFiles[0], path.extname(entryFiles[0])) + '.torrent'
            : entryName + '.torrent');

    const iceServers = entryIce.length
        ? entryIce
        : [{ urls: 'stun:stun.l.google.com:19302' }, { urls: 'stun:stun1.l.google.com:19302' }];

    const client = new RtcTorrent({ trackerUrl: entryTracker, rtcInterval: entryInterval, iceServers });

    const entryVersion = entry.version || 'v1';
    process.stdout.write(`[${entryName}] Hashing pieces (${entryVersion})… `);
    let result;
    try {
        result = await client.create(entryFiles, {
            name: entryName,
            version: entryVersion,
            webseedUrls: entryWebseeds.length ? entryWebseeds : undefined,
        });
    } catch (e) {
        console.error(`\n[${entryName}] Failed to create torrent: ${e.message}`);
        return;
    }

    const { infoHash, encodedTorrent, magnetUri, v2InfoHash } = result;
    console.log('done.');
    fs.writeFileSync(entryOut, Buffer.isBuffer(encodedTorrent) ? encodedTorrent : Buffer.from(encodedTorrent));
    console.log(`[${entryName}] Saved  : ${entryOut}`);
    console.log(`[${entryName}] Hash   : ${infoHash}`);
    if (v2InfoHash) console.log(`[${entryName}] v2Hash : ${v2InfoHash}`);
    console.log(`[${entryName}] Magnet : ${magnetUri}`);

    await client.seed(encodedTorrent, entryFiles);
    console.log(`[${entryName}] Seeding…`);

    setInterval(() => {
        const torrent = [...client.torrents.values()][0];
        if (!torrent) return;
        console.log(
            `[${new Date().toLocaleTimeString()}][${entryName}] ` +
            `peers: ${torrent.peers.size}  uploaded: ${fmt(torrent.uploaded)}`
        );
    }, 10000);
}

/**
 * Run all torrents from the YAML file concurrently.
 *
 * Reload behaviour:
 *   When the YAML file is modified (or SIGHUP received on Unix), the process
 *   exits with code 0 so a process manager can restart it with the new config.
 */
async function runTorrentsMode(yamlFile) {
    console.log('=== RtcTorrent Seeder (multi-torrent mode) ===');
    console.log(`Config  : ${yamlFile}\n`);

    let config;
    try {
        config = loadYaml(yamlFile);
    } catch (e) {
        console.error('Failed to load YAML:', e.message);
        process.exit(1);
    }

    const entries = (config && config.torrents) || [];
    if (entries.length === 0) {
        console.error('No torrent entries found in YAML file.');
        process.exit(1);
    }

    console.log(`Starting ${entries.length} torrent(s)…\n`);

    // ── Reload handler ──────────────────────────────────────────────────────
    // On reload: exit cleanly (code 0) and let the process manager restart.
    function onReload(reason) {
        console.log(`\n[Reload] ${reason}`);
        console.log('[Reload] Exiting with code 0 — restart the process to apply new config.');
        process.exit(0);
    }

    // SIGHUP — Unix/macOS only (not available on Windows)
    if (process.platform !== 'win32') {
        process.on('SIGHUP', () => onReload('SIGHUP received'));
    }

    // File-modification polling — cross-platform (including Windows)
    let lastMtime = fs.statSync(yamlFile).mtimeMs;
    setInterval(() => {
        try {
            const mtime = fs.statSync(yamlFile).mtimeMs;
            if (mtime !== lastMtime) {
                lastMtime = mtime;
                onReload('Config file changed on disk');
            }
        } catch (_) {
            // File temporarily inaccessible during save — ignore
        }
    }, 2000);

    // ── Start all seeders concurrently ──────────────────────────────────────
    await Promise.all(
        entries.map((entry, i) =>
            seedEntry(entry).catch(e => console.error(`[entry-${i}] Fatal: ${e.message}`))
        )
    );
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

function fmt(bytes) {
    if (bytes < 1024)        return bytes + ' B';
    if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB';
    if (bytes < 1024 ** 3)   return (bytes / 1024 / 1024).toFixed(1) + ' MB';
    return (bytes / 1024 / 1024 / 1024).toFixed(2) + ' GB';
}
