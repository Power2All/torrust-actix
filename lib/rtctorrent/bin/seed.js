#!/usr/bin/env node
/**
 * RtcTorrent CLI seeder
 *
 * Single-torrent mode:
 *   node seed.js [--tracker <url>] ... [--name <name>] [--out <path>]
 *                [--webseed <url>] ... [--torrent-version v1|v2|hybrid]
 *                <file1> [<file2> ...]
 *
 *   Re-seed from an existing .torrent (no re-hashing):
 *   node seed.js [--tracker <url>] ... --torrent-file <path> [<file1> ...]
 *
 *   Seed using a magnet URI (tracker URLs read from it):
 *   node seed.js [--tracker <url>] ... --magnet <uri> <file1> [<file2> ...]
 *
 * Multi-torrent mode (YAML):
 *   node seed.js --torrents <config.yaml>
 *
 * YAML format:
 *   ---
 *   torrents:
 *     - out: "/path/to/output.torrent"       # optional
 *       name: "My Movie"                      # optional
 *       file:
 *         - "/path/to/movie.mp4"
 *       trackers:                             # optional
 *         - "http://localhost:6969/announce"
 *         - "http://tracker2.example.com/announce"
 *       torrent_file: "/path/movie.torrent"   # optional — re-seed without re-hashing
 *       magnet: "magnet:?xt=..."              # optional — read trackers from magnet
 *       webseed:                              # optional
 *         - "https://cdn.example.com/movie.mp4"
 *       ice:                                  # optional
 *         - "stun:stun.l.google.com:19302"
 *       rtc_interval: 10000                   # optional, milliseconds
 *       version: v1                           # optional: v1 | v2 | hybrid
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
const trackerUrls   = [];     // --tracker (repeatable, optional)
let torrentFile     = null;   // --torrent-file
let magnetUri       = null;   // --magnet
let torrentName     = null;
let outFile         = null;
let torrentVersion  = 'v1';   // --torrent-version: v1 | v2 | hybrid
const filePaths     = [];
const webseedUrls   = [];

for (let i = 0; i < args.length; i++) {
    if      (args[i] === '--torrents'        && args[i + 1]) { torrentsFile = args[++i]; }
    else if (args[i] === '--tracker'         && args[i + 1]) { trackerUrls.push(args[++i]); }
    else if (args[i] === '--torrent-file'    && args[i + 1]) { torrentFile  = args[++i]; }
    else if (args[i] === '--magnet'          && args[i + 1]) { magnetUri    = args[++i]; }
    else if (args[i] === '--name'            && args[i + 1]) { torrentName  = args[++i]; }
    else if (args[i] === '--out'             && args[i + 1]) { outFile      = args[++i]; }
    else if (args[i] === '--webseed'         && args[i + 1]) { webseedUrls.push(args[++i]); }
    else if (args[i] === '--torrent-version' && args[i + 1]) { torrentVersion = args[++i]; }
    else if (!args[i].startsWith('--'))                      { filePaths.push(args[i]); }
}

// ─── Mode dispatch ───────────────────────────────────────────────────────────

if (torrentsFile) {
    // Validate: no single-torrent flags allowed alongside --torrents
    if (filePaths.length > 0 || torrentName || outFile || webseedUrls.length > 0 ||
        torrentFile || magnetUri || trackerUrls.length > 0) {
        console.error(
            'Error: --torrents cannot be combined with single-torrent options ' +
            '(positional files, --tracker, --torrent-file, --magnet, --name, --out, --webseed).'
        );
        process.exit(1);
    }
    if (!fs.existsSync(torrentsFile)) {
        console.error(`Torrents file not found: ${torrentsFile}`);
        process.exit(1);
    }
    runTorrentsMode(torrentsFile).catch(e => { console.error('Fatal:', e); process.exit(1); });
} else {
    // Files are required unless --torrent-file is given
    if (filePaths.length === 0 && !torrentFile) {
        console.error(
            'Usage:\n' +
            '  node seed.js [--tracker <url>] [--name <name>] [--webseed <url>] <file1> ...\n' +
            '  node seed.js [--tracker <url>] --torrent-file <path> [<file1> ...]\n' +
            '  node seed.js [--tracker <url>] --magnet <uri> <file1> ...\n' +
            '  node seed.js --torrents <config.yaml>'
        );
        process.exit(1);
    }
    for (const p of filePaths) {
        if (!fs.existsSync(p)) { console.error(`File not found: ${p}`); process.exit(1); }
    }
    if (torrentFile && !fs.existsSync(torrentFile)) {
        console.error(`Torrent file not found: ${torrentFile}`);
        process.exit(1);
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

// ─── Helpers ─────────────────────────────────────────────────────────────────

function fmt(bytes) {
    if (bytes < 1024)        return bytes + ' B';
    if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB';
    if (bytes < 1024 ** 3)   return (bytes / 1024 / 1024).toFixed(1) + ' MB';
    return (bytes / 1024 / 1024 / 1024).toFixed(2) + ' GB';
}

/** Format a list of tracker URLs for console display. */
function fmtTrackers(urls) {
    if (urls.length === 0) return '(none — seeding without announcing)';
    return urls.join('\n          ');
}

/**
 * Build a magnet URI with multiple tracker URLs.
 * version: 'v1' | 'v2' | 'hybrid'
 * v2InfoHash: full 64-char SHA-256 hex (used for urn:btmh:1220)
 */
function buildMagnetUri(infoHash, v2InfoHash, name, allTrackers, version) {
    let uri = 'magnet:?';
    if (version === 'v2' && v2InfoHash) {
        uri += `xt=urn:btmh:1220${v2InfoHash}`;
    } else if (version === 'hybrid' && v2InfoHash) {
        uri += `xt=urn:btih:${infoHash}&xt=urn:btmh:1220${v2InfoHash}`;
    } else {
        uri += `xt=urn:btih:${infoHash}`;
    }
    uri += `&dn=${encodeURIComponent(name)}`;
    for (const url of allTrackers) {
        uri += `&tr=${encodeURIComponent(url)}`;
    }
    return uri;
}

/**
 * Collect all tracker URLs from a parsed torrent object
 * (announce + all tiers of announce-list).
 */
function trackersFromParsed(parsed) {
    const seen = new Set();
    const out = [];
    const add = u => {
        const s = Buffer.isBuffer(u) ? u.toString() : String(u);
        if (s && !seen.has(s)) { seen.add(s); out.push(s); }
    };
    if (parsed.announce) {
        const ann = parsed.announce;
        Array.isArray(ann) ? ann.forEach(add) : add(ann);
    }
    if (Array.isArray(parsed.announceList)) {
        for (const tier of parsed.announceList) {
            if (Array.isArray(tier)) tier.forEach(add);
            else add(tier);
        }
    }
    return out;
}

/**
 * Resolve seeding file paths from a parsed torrent's info dict when the user
 * didn't provide explicit file paths on the command line.
 */
function inferFilePaths(parsedInfo) {
    const toStr = v => Buffer.isBuffer(v) ? v.toString() : String(v);
    const name = toStr(parsedInfo.name);
    if (parsedInfo.files && Array.isArray(parsedInfo.files)) {
        // Multi-file torrent
        return parsedInfo.files.map(f => {
            const parts = Array.isArray(f.path)
                ? f.path.map(toStr)
                : [toStr(f.path)];
            return path.join(process.cwd(), name, ...parts);
        });
    }
    // Single-file torrent
    return [path.join(process.cwd(), name)];
}

// ─── Single-torrent mode ─────────────────────────────────────────────────────

async function main() {
    const RtcTorrent = loadLib();

    // ── --torrent-file: re-seed without re-hashing ──────────────────────────
    if (torrentFile) {
        const torrentBytes = fs.readFileSync(torrentFile);
        const client = new RtcTorrent({
            trackerUrl: trackerUrls[0] || '',
            rtcInterval: 5000,
            iceServers: [
                { urls: 'stun:stun.l.google.com:19302' },
                { urls: 'stun:stun1.l.google.com:19302' }
            ]
        });

        let parsed;
        try {
            parsed = await client.parseTorrentFile(torrentBytes);
        } catch (e) {
            console.error('Failed to parse torrent file:', e.message);
            process.exit(1);
        }

        const name = torrentName || parsed.name || path.basename(torrentFile, path.extname(torrentFile));

        // Merge: --tracker flags first, then trackers from .torrent file
        const fromFile = trackersFromParsed(parsed);
        const allTrackers = [...new Set([...trackerUrls, ...fromFile])];

        // Resolve file paths: explicit args > inferred from torrent info
        const seedFiles = filePaths.length > 0
            ? filePaths
            : (parsed.info ? inferFilePaths(parsed.info) : []);

        console.log('=== RtcTorrent Seeder ===');
        console.log(`Trackers: ${fmtTrackers(allTrackers)}`);
        console.log(`Files   : ${seedFiles.join(', ')}`);
        console.log('');
        console.log(`Hash  : ${parsed.infoHash}`);
        if (parsed.v2InfoHash) console.log(`v2Hash: ${parsed.v2InfoHash}`);
        console.log('\nRe-seeding from existing .torrent — no re-hashing needed.');
        console.log('Share the .torrent file with leechers.\n');

        // Inject all trackers into the parsed object for the seeding announce loop
        parsed.announce = allTrackers[0] || '';
        parsed.announceList = allTrackers.length > 0 ? [allTrackers] : [];

        await client.seed(parsed, seedFiles);
        console.log('Seeding… (Ctrl+C to stop)\n');

        setInterval(() => {
            const torrent = [...client.torrents.values()][0];
            if (!torrent) return;
            console.log(
                `[${new Date().toLocaleTimeString()}] peers: ${torrent.peers.size}` +
                `  uploaded: ${fmt(torrent.uploaded)}`
            );
        }, 10000);
        return;
    }

    // ── --magnet: extract tracker URLs and optional name from magnet URI ─────
    let magnetTrackers = [];
    let magnetName = null;
    if (magnetUri) {
        try {
            const url = new URL(magnetUri);
            magnetTrackers = url.searchParams.getAll('tr');
            const dn = url.searchParams.get('dn');
            if (dn) magnetName = decodeURIComponent(dn);
        } catch (e) {
            console.error('Failed to parse magnet URI:', e.message);
            process.exit(1);
        }
    }

    // ── Common create() path ─────────────────────────────────────────────────

    // Resolve name and output path
    if (filePaths.length === 1 && !torrentName) {
        torrentName = magnetName || path.basename(filePaths[0]);
        outFile = outFile || path.basename(filePaths[0], path.extname(filePaths[0])) + '.torrent';
    } else {
        torrentName = torrentName || magnetName || path.basename(filePaths[0], path.extname(filePaths[0]));
        outFile = outFile || `${torrentName}.torrent`;
    }

    // Combine all tracker URLs (--tracker flags + from magnet, deduped)
    const allTrackers = [...new Set([...trackerUrls, ...magnetTrackers])];
    const firstTracker = allTrackers[0] || '';

    console.log('=== RtcTorrent Seeder ===');
    console.log(`Trackers: ${fmtTrackers(allTrackers)}`);
    console.log(`Files   : ${filePaths.join(', ')}`);
    if (webseedUrls.length) console.log(`Webseeds: ${webseedUrls.join(', ')}`);
    console.log('');

    const client = new RtcTorrent({
        trackerUrl: firstTracker,
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

    const { infoHash, encodedTorrent, v2InfoHash } = result;
    console.log('done.');
    fs.writeFileSync(outFile, Buffer.isBuffer(encodedTorrent) ? encodedTorrent : Buffer.from(encodedTorrent));
    console.log(`\nSaved : ${outFile}`);
    console.log(`Hash  : ${infoHash}`);
    if (v2InfoHash) console.log(`v2Hash: ${v2InfoHash}`);

    const magnetUriOut = buildMagnetUri(infoHash, v2InfoHash, torrentName, allTrackers, torrentVersion);
    console.log(`\nMagnet URI:\n${magnetUriOut}\n`);
    console.log('Share the magnet URI or the .torrent file with leechers.\n');

    // Inject all trackers into result for the seeding announce loop
    result.announce = firstTracker;
    result.announceList = allTrackers.length > 0 ? [allTrackers] : [];

    await client.seed(result, filePaths);
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

    const entryTrackers    = entry.trackers     || [];
    const entryTorrentFile = entry.torrent_file || null;
    const entryMagnet      = entry.magnet       || null;
    const entryFiles       = entry.file         || [];
    const entryWebseeds    = entry.webseed      || [];
    const entryIce         = (entry.ice || []).map(u => ({ urls: u }));
    const entryInterval    = entry.rtc_interval || 5000;
    const entryVersion     = entry.version      || 'v1';

    if (entryFiles.length === 0 && !entryTorrentFile) {
        throw new Error('torrent entry has no files and no torrent_file');
    }
    for (const p of entryFiles) {
        if (!fs.existsSync(p)) throw new Error(`File not found: ${p}`);
    }
    if (entryTorrentFile && !fs.existsSync(entryTorrentFile)) {
        throw new Error(`Torrent file not found: ${entryTorrentFile}`);
    }

    const iceServers = entryIce.length
        ? entryIce
        : [{ urls: 'stun:stun.l.google.com:19302' }, { urls: 'stun:stun1.l.google.com:19302' }];

    // ── --torrent-file case: re-seed without re-hashing ──────────────────────
    if (entryTorrentFile) {
        const torrentBytes = fs.readFileSync(entryTorrentFile);
        const client = new RtcTorrent({
            trackerUrl: entryTrackers[0] || '',
            rtcInterval: entryInterval,
            iceServers
        });

        let parsed;
        try {
            parsed = await client.parseTorrentFile(torrentBytes);
        } catch (e) {
            console.error(`[torrent_file] Failed to parse: ${e.message}`);
            return;
        }

        const entryName = entry.name || parsed.name ||
            path.basename(entryTorrentFile, path.extname(entryTorrentFile));

        // Merge: YAML trackers first, then trackers from .torrent file
        const fromFile = trackersFromParsed(parsed);
        const allTrackers = [...new Set([...entryTrackers, ...fromFile])];

        const seedFiles = entryFiles.length > 0
            ? entryFiles
            : (parsed.info ? inferFilePaths(parsed.info) : []);

        console.log(`[${entryName}] Re-seeding from .torrent — no re-hashing needed.`);
        console.log(`[${entryName}] Hash   : ${parsed.infoHash}`);
        if (parsed.v2InfoHash) console.log(`[${entryName}] v2Hash : ${parsed.v2InfoHash}`);

        parsed.announce = allTrackers[0] || '';
        parsed.announceList = allTrackers.length > 0 ? [allTrackers] : [];

        await client.seed(parsed, seedFiles);
        console.log(`[${entryName}] Seeding…`);

        setInterval(() => {
            const torrent = [...client.torrents.values()][0];
            if (!torrent) return;
            console.log(
                `[${new Date().toLocaleTimeString()}][${entryName}] ` +
                `peers: ${torrent.peers.size}  uploaded: ${fmt(torrent.uploaded)}`
            );
        }, 10000);
        return;
    }

    // ── --magnet case: extract tracker URLs from magnet URI ───────────────────
    let magnetTrackers = [];
    let magnetName = null;
    if (entryMagnet) {
        try {
            const url = new URL(entryMagnet);
            magnetTrackers = url.searchParams.getAll('tr');
            const dn = url.searchParams.get('dn');
            if (dn) magnetName = decodeURIComponent(dn);
        } catch (e) {
            console.error(`[entry] Failed to parse magnet URI: ${e.message}`);
            return;
        }
    }

    const allTrackers = [...new Set([...entryTrackers, ...magnetTrackers])];
    const firstTracker = allTrackers[0] || '';

    const entryName = entry.name || magnetName ||
        (entryFiles.length === 1
            ? path.basename(entryFiles[0])
            : path.basename(entryFiles[0], path.extname(entryFiles[0])));
    const entryOut = entry.out ||
        (entryFiles.length === 1
            ? path.basename(entryFiles[0], path.extname(entryFiles[0])) + '.torrent'
            : entryName + '.torrent');

    const client = new RtcTorrent({ trackerUrl: firstTracker, rtcInterval: entryInterval, iceServers });

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

    const { infoHash, encodedTorrent, v2InfoHash } = result;
    console.log('done.');
    fs.writeFileSync(entryOut, Buffer.isBuffer(encodedTorrent) ? encodedTorrent : Buffer.from(encodedTorrent));
    console.log(`[${entryName}] Saved  : ${entryOut}`);
    console.log(`[${entryName}] Hash   : ${infoHash}`);
    if (v2InfoHash) console.log(`[${entryName}] v2Hash : ${v2InfoHash}`);
    const magnetUriOut = buildMagnetUri(infoHash, v2InfoHash, entryName, allTrackers, entryVersion);
    console.log(`[${entryName}] Magnet : ${magnetUriOut}`);

    result.announce = firstTracker;
    result.announceList = allTrackers.length > 0 ? [allTrackers] : [];

    await client.seed(result, entryFiles);
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
