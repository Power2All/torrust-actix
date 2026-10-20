#!/usr/bin/env node
/**
 * Static file server for the RtcTorrent browser demo.
 *
 * Serves the lib/rtctorrent/ directory so the demo page can load
 * the built browser bundle via a relative path (../dist/...).
 *
 * Usage:
 *   node bin/serve.js [port]
 *
 * Default port: 8080
 * Open: http://localhost:8080/demo/
 */
'use strict';

const http = require('http');
const fs   = require('fs');
const path = require('path');
const PORT = parseInt(process.argv[2] || '8080', 10);
const ROOT = path.join(__dirname, '..');
const MIME = {
    '.html': 'text/html',
    '.js':   'application/javascript',
    '.css':  'text/css',
    '.json': 'application/json',
    '.png':  'image/png',
    '.jpg':  'image/jpeg',
    '.svg':  'image/svg+xml',
    '.ico':  'image/x-icon',
    '.mp4':  'video/mp4',
    '.webm': 'video/webm',
    '.torrent': 'application/x-bittorrent',
};
const server = http.createServer((req, res) => {
    let urlPath = req.url.split('?')[0];
    if (urlPath === '/' || urlPath === '') urlPath = '/demo/index.html';
    if (urlPath.endsWith('/')) urlPath += 'index.html';
    const filePath = path.join(ROOT, urlPath);
    if (!filePath.startsWith(ROOT + path.sep) && filePath !== ROOT) {
        res.writeHead(403);
        res.end('Forbidden');
        return;
    }
    fs.stat(filePath, (err, stat) => {
        if (err || !stat.isFile()) {
            const missing = !err || err.code === 'ENOENT';
            res.writeHead(missing ? 404 : 500);
            res.end(missing ? 'Not found' : 'Server error');
            return;
        }
        const ext  = path.extname(filePath).toLowerCase();
        const mime = MIME[ext] || 'application/octet-stream';
        // Streamed, with single byte ranges: video seeking and the library's webseed fetch both
        // send `Range`, and a whole-file 200 gave every piece after the first the wrong bytes.
        const range = /^bytes=(\d*)-(\d*)$/.exec(req.headers.range || '');
        if (range && (range[1] || range[2])) {
            const start = range[1] ? Number(range[1]) : Math.max(0, stat.size - Number(range[2]));
            const end = range[1] && range[2] ? Math.min(Number(range[2]), stat.size - 1) : stat.size - 1;
            if (start > end) {
                res.writeHead(416, { 'Content-Range': `bytes */${stat.size}` });
                res.end();
                return;
            }
            res.writeHead(206, {
                'Content-Type': mime,
                'Content-Length': end - start + 1,
                'Content-Range': `bytes ${start}-${end}/${stat.size}`,
                'Accept-Ranges': 'bytes',
            });
            fs.createReadStream(filePath, { start, end }).pipe(res);
            return;
        }
        res.writeHead(200, { 'Content-Type': mime, 'Content-Length': stat.size, 'Accept-Ranges': 'bytes' });
        fs.createReadStream(filePath).pipe(res);
    });
});
// Loopback only: this is a development server, and the demo needs a secure context
// (`crypto.subtle`), which plain http only gets on localhost anyway.
server.listen(PORT, '127.0.0.1', () => {
    console.log(`RtcTorrent demo server running at http://localhost:${PORT}/demo/`);
    console.log('Press Ctrl+C to stop.');
});