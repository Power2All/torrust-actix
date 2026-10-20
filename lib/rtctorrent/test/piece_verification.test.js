'use strict';
// Pieces come from peers nobody vouched for: a tampered or out-of-range piece must be rejected
// for every torrent format the library creates. Before v4.2.24 pure v2 accepted anything.
const fs = require('fs');
const os = require('os');
const path = require('path');
const RtcTorrent = require('../dist/rtctorrent.node.js');

const content = Buffer.alloc(100002); // 7 pieces of 16 KiB, the last one short
for (let i = 0; i < content.length; i++) content[i] = (i * 31) & 0xff;
const file = path.join(os.tmpdir(), `rtcverify_${process.pid}.bin`);

beforeAll(() => fs.writeFileSync(file, content));
afterAll(() => fs.unlinkSync(file));

test.each(['v1', 'hybrid', 'v2'])('%s pieces are verified', async (version) => {
    const client = new RtcTorrent({ trackerUrl: 'http://127.0.0.1:9/announce', rtcInterval: 1e9 });
    const created = await client.create([file], { name: `verify-${version}`, version });
    await client.download(created.encodedTorrent);
    const torrent = [...client.torrents.values()][0];
    try {
        const piece = i => content.subarray(i * torrent.pieceLength, Math.min((i + 1) * torrent.pieceLength, content.length));
        expect(torrent.pieceCount).toBe(7);
        for (let i = 0; i < torrent.pieceCount; i++) {
            expect(await torrent._verifyPiece(i, piece(i))).toBe(true);
            const tampered = Buffer.from(piece(i));
            tampered[0] ^= 1;
            expect(await torrent._verifyPiece(i, tampered)).toBe(false);
        }
        expect(await torrent._verifyPiece(torrent.pieceCount, piece(0))).toBe(false);
    } finally {
        await torrent.stop();
    }
});

test('a peer cannot queue piece requests without limit', async () => {
    const client = new RtcTorrent({ trackerUrl: 'http://127.0.0.1:9/announce', rtcInterval: 1e9 });
    const created = await client.create([file], { name: 'queue-cap' });
    await client.download(created.encodedTorrent);
    const torrent = [...client.torrents.values()][0];
    try {
        const channel = { readyState: 'open', bufferedAmount: 0, send() {} };
        for (let i = 0; i < 10000; i++) torrent.handlePieceRequest(i % torrent.pieceCount, channel);
        torrent.handlePieceRequest(torrent.pieceCount, channel);
        expect(channel._queued).toBe(128);
        await channel._sendQueue;
        expect(channel._queued).toBe(0);
    } finally {
        await torrent.stop();
    }
});

test('v2 roots follow BEP 52 (a short final block is hashed as it is)', async () => {
    // Computed independently: SHA-256 per 16 KiB block (the last one unpadded), zero leaves up to
    // a power of two, then pairwise SHA-256 up to the root, as libtorrent does.
    const crypto = require('crypto');
    const sha256 = b => crypto.createHash('sha256').update(b).digest();
    let level = [];
    for (let o = 0; o < content.length; o += 16384) level.push(sha256(content.subarray(o, o + 16384)));
    let width = 1;
    while (width < level.length) width *= 2;
    while (level.length < width) level.push(Buffer.alloc(32));
    while (level.length > 1) {
        const next = [];
        for (let i = 0; i < level.length; i += 2) next.push(sha256(Buffer.concat([level[i], level[i + 1]])));
        level = next;
    }
    const client = new RtcTorrent({ trackerUrl: 'http://127.0.0.1:9/announce', rtcInterval: 1e9 });
    const created = await client.create([file], { name: 'bep52', version: 'v2' });
    const parsed = await client.parseTorrentFile(created.encodedTorrent);
    const entry = Object.values(parsed.info['file tree'])[0][''];
    expect(Buffer.from(entry['pieces root']).equals(level[0])).toBe(true);
});

test.each(['v2', 'hybrid'])('multi-file %s pieces are laid out per file and verified', async (version) => {
    // Given in unsorted order, none a whole number of 16 KiB pieces: the v2 file tree sorts them
    // and starts each on a piece boundary (a hybrid's v1 list too, with BEP 47 pad files), which
    // seeder and leecher must both follow. libtorrent 2.1 accepts and fully verifies these.
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), `rtc${version}-`));
    const sizes = { 'c.bin': 40000, 'a.bin': 5000, 'b.bin': 16384 * 2 + 7 };
    const paths = Object.entries(sizes).map(([name, size]) => {
        const p = path.join(dir, name);
        fs.writeFileSync(p, Buffer.alloc(size, name.charCodeAt(0)));
        return p;
    });
    const seeder = new RtcTorrent({ trackerUrl: 'http://127.0.0.1:9/announce', rtcInterval: 1e9 });
    const created = await seeder.create(paths, { name: `multi-${version}`, version });
    const leecher = new RtcTorrent({ trackerUrl: 'http://127.0.0.1:9/announce', rtcInterval: 1e9 });
    await leecher.download(created.encodedTorrent);
    const verifying = [...leecher.torrents.values()][0];
    const fromDisk = new RtcTorrent({ trackerUrl: 'http://127.0.0.1:9/announce', rtcInterval: 1e9 });
    try {
        expect(verifying.hasGaps).toBe(true);
        if (version === 'hybrid') {
            const v1 = verifying.data.info.files.map(f => [Buffer.from(f.path[f.path.length - 1]).toString(), f.attr ? Buffer.from(f.attr).toString() : '']);
            expect(v1).toEqual([['a.bin', ''], ['11384', 'p'], ['b.bin', ''], ['16377', 'p'], ['c.bin', '']]);
        }
        expect(verifying.pieceCount).toBe(1 + 3 + 3); // a: 1, b: 3, c: 3
        const memory = await seeder.seed(created);
        const disk = await fromDisk.seed(created.encodedTorrent, paths);
        for (const source of [memory, disk]) {
            for (let i = 0; i < verifying.pieceCount; i++) {
                const piece = Buffer.from(await source._readPieceAsync(i));
                expect(await verifying._verifyPiece(i, piece)).toBe(true);
                piece[0] ^= 1;
                expect(await verifying._verifyPiece(i, piece)).toBe(false);
            }
        }
    } finally {
        for (const c of [seeder, leecher, fromDisk]) for (const t of c.torrents.values()) await t.stop();
        fs.rmSync(dir, { recursive: true, force: true });
    }
});
