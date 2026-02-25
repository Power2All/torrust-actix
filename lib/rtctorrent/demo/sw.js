'use strict';

/**
 * Service Worker for seamless video streaming.
 *
 * The key insight: Chrome sends `Range: bytes=0-` as its FIRST request for any
 * video URL. If we return 416 for that probe, Chrome immediately fires
 * MEDIA_ERR_SRC_NOT_SUPPORTED (code 4) and gives up entirely.
 *
 * Strategy:
 *  - The FIRST request to a streamId — Range or not — always gets 200 + full
 *    ReadableStream + Accept-Ranges: none.
 *  - Accept-Ranges: none tells Chrome "no seeking, read sequentially", so it
 *    won't send further Range requests during streaming.
 *  - Any SUBSEQUENT request to the same streamId (duplicate) gets 416 so the
 *    browser sticks to reading the already-established stream.
 */

const streamControllers = new Map();
const pendingChunks = new Map(); // streamId → [{type, data?}]

self.addEventListener('install', () => self.skipWaiting());
self.addEventListener('activate', event => event.waitUntil(self.clients.claim()));

self.addEventListener('fetch', event => {
    const url = new URL(event.request.url);
    if (!url.pathname.includes('/__rtc_stream__/')) return;
    const streamId = url.pathname.split('/__rtc_stream__/')[1];
    const mime = url.searchParams.get('mime') || 'video/mp4';
    const size = url.searchParams.get('size');
    if (streamControllers.has(streamId)) {
        event.respondWith(new Response(null, {
            status: 416,
            statusText: 'Range Not Satisfiable',
            headers: { 'Content-Range': `bytes */${size || '*'}` }
        }));
        return;
    }
    const headers = { 'Content-Type': mime, 'Accept-Ranges': 'none' };
    if (size) headers['Content-Length'] = size;
    const body = new ReadableStream({
        start(controller) {
            streamControllers.set(streamId, controller);
            const pending = pendingChunks.get(streamId);
            if (pending) {
                for (const item of pending) {
                    if (item.type === 'chunk') controller.enqueue(item.data);
                    else if (item.type === 'end')  { controller.close(); break; }
                }
                pendingChunks.delete(streamId);
            }
        },
        cancel() {
            streamControllers.delete(streamId);
        }
    });
    event.respondWith(new Response(body, { status: 200, headers }));
});

self.addEventListener('message', event => {
    const { type, streamId, chunk } = event.data;
    const controller = streamControllers.get(streamId);
    if (!controller) {
        if (!pendingChunks.has(streamId)) pendingChunks.set(streamId, []);
        if (type === 'chunk') pendingChunks.get(streamId).push({ type: 'chunk', data: new Uint8Array(chunk) });
        else if (type === 'end') pendingChunks.get(streamId).push({ type: 'end' });
        return;
    }
    if (type === 'chunk') {
        controller.enqueue(new Uint8Array(chunk));
    } else if (type === 'end') {
        controller.close();
        streamControllers.delete(streamId);
    }
});