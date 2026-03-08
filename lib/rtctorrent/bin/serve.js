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
    fs.readFile(filePath, (err, data) => {
        if (err) {
            res.writeHead(err.code === 'ENOENT' ? 404 : 500);
            res.end(err.code === 'ENOENT' ? 'Not found' : 'Server error');
            return;
        }
        const ext  = path.extname(filePath).toLowerCase();
        const mime = MIME[ext] || 'application/octet-stream';
        res.writeHead(200, { 'Content-Type': mime });
        res.end(data);
    });
});

server.listen(PORT, () => {
    console.log(`RtcTorrent demo server running at http://localhost:${PORT}/demo/`);
    console.log('Press Ctrl+C to stop.');
});