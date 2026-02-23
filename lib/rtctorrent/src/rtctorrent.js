/**
 * RtcTorrent - WebRTC-enabled BitTorrent client library
 * Enables WebRTC-based peer-to-peer connections for BitTorrent downloads
 * using the HTTP/HTTPS announce endpoint for signaling (no WebSocket needed).
 *
 * Signaling protocol:
 *  1. Seeder creates RTCPeerConnection + data channel, generates SDP offer (with ICE),
 *     announces with rtctorrent=1&rtcoffer=<url-encoded-SDP>.
 *  2. Leecher announces with rtctorrent=1&rtcrequest=1, receives rtc_peers list
 *     containing seeders with their SDP offers.
 *  3. Leecher creates RTCPeerConnection, sets remote description (seeder offer),
 *     creates SDP answer (with ICE), announces back with
 *     rtctorrent=1&rtcanswer=<SDP>&rtcanswerfor=<seeder-peer-id-hex>.
 *  4. Seeder polls tracker, receives pending rtc_answers, sets remote description,
 *     and the WebRTC data channel opens on both ends.
 */
let _fetch;
if (typeof fetch !== 'undefined') {
    _fetch = fetch.bind(globalThis);
} else {
    _fetch = function (url, _opts) {
        return new Promise((resolve, reject) => {
            const mod = url.startsWith('https') ? require('https') : require('http');
            mod.get(url, (res) => {
                const chunks = [];
                res.on('data', (c) => chunks.push(c));
                res.on('end', () => {
                    const buf = Buffer.concat(chunks);
                    resolve({
                        ok: res.statusCode >= 200 && res.statusCode < 300,
                        status: res.statusCode,
                        arrayBuffer: () => {
                            const ab = buf.buffer.slice(buf.byteOffset, buf.byteOffset + buf.byteLength);
                            return Promise.resolve(ab);
                        }
                    });
                });
            }).on('error', reject);
        });
    };
}
const MSG_PIECE_REQUEST = 0x01;
const MSG_PIECE_DATA = 0x02;
const MSG_HAVE = 0x03;
const MAX_IN_FLIGHT = 16;

class RtcTorrent {
    constructor(options = {}) {
        this.options = {
            trackerUrl: options.trackerUrl || '',
            announceInterval: options.announceInterval || 30000,
            rtcInterval: options.rtcInterval || 10000,
            maxPeers: options.maxPeers || 50,
            iceServers: options.iceServers || [
                { urls: 'stun:stun.l.google.com:19302' },
                { urls: 'stun:stun1.l.google.com:19302' }
            ],
            ...options
        };
        this.pieces = new Map();
        this.downloaded = 0;
        this.requestedPieces = new Set();
        this.torrents = new Map();
        this.connections = new Map();
        this.announcer = null;
        this.webRtcManager = null;
        this.bencoder = null;
        this.isBrowser = typeof window !== 'undefined';
        this.isFileApiAvailable = this.isBrowser && typeof File !== 'undefined';
        if (!this.isBrowser) {
            try { this.fs = require('fs'); } catch (_) {}
            try { this.path = require('path'); } catch (_) {}
        }
    }

    getRTCPeerConnection() {
        if (typeof RTCPeerConnection !== 'undefined') {
            return RTCPeerConnection;
        }
        const nativeRequire = (typeof __non_webpack_require__ !== 'undefined')
            ? __non_webpack_require__
            : require;
        for (const pkg of ['wrtc', '@roamhq/wrtc', 'node-webrtc']) {
            try {
                const m = nativeRequire(pkg);
                if (m.RTCPeerConnection) return m.RTCPeerConnection;
            } catch (_) {}
        }
        return null;
    }

    async initBencoder() {
        if (this.bencoder) return this.bencoder;
        try {
            this.bencoder = require('bencode');
        } catch (_) {
            this.bencoder = this.createMinimalBencoder();
        }
        return this.bencoder;
    }

    createMinimalBencoder() {
        const enc = (v) => {
            if (typeof v === 'string') return `${v.length}:${v}`;
            if (typeof v === 'number') return `i${v}e`;
            if (Buffer.isBuffer(v) || v instanceof Uint8Array) {
                const b = Buffer.isBuffer(v) ? v : Buffer.from(v);
                return `${b.length}:${b.toString('binary')}`;
            }
            if (Array.isArray(v)) return 'l' + v.map(enc).join('') + 'e';
            if (v && typeof v === 'object') {
                return 'd' + Object.keys(v).sort().map(k => enc(k) + enc(v[k])).join('') + 'e';
            }
            return '';
        };
        return { encode: enc };
    }

    async create(files, options = {}) {
        const bencoder = await this.initBencoder();
        let totalSize = 0, fileInfos = [], allFileBuffers = [];
        if (Array.isArray(files)) {
            for (const file of files) {
                if (!this.isBrowser && this.fs) {
                    const filePath = typeof file === 'string' ? file : file.path;
                    const fileName = typeof file === 'string'
                        ? this.path.basename(filePath)
                        : (file.name || this.path.basename(filePath));
                    const stats = this.fs.statSync(filePath);
                    totalSize += stats.size;
                    fileInfos.push({ length: stats.size, path: [fileName] });
                    allFileBuffers.push(this.fs.readFileSync(filePath));
                } else if (this.isBrowser && this.isFileApiAvailable && file instanceof File) {
                    totalSize += file.size;
                    fileInfos.push({ length: file.size, path: [file.name] });
                    const ab = await file.arrayBuffer();
                    allFileBuffers.push(Buffer.from(ab));
                } else {
                    totalSize += file.size || 0;
                    fileInfos.push({ length: file.size || 0, path: [file.name || 'file'] });
                }
            }
        }
        let pieceLength = 16 * 1024;
        if (totalSize > 8  * 1024 * 1024) pieceLength = 32 * 1024;
        if (totalSize > 64 * 1024 * 1024) pieceLength = 64 * 1024;
        const combinedBuffer = allFileBuffers.length > 0
            ? Buffer.concat(allFileBuffers)
            : Buffer.alloc(totalSize);
        const numPieces = Math.ceil(totalSize / pieceLength);
        const piecesArray = new Uint8Array(numPieces * 20);
        for (let i = 0; i < numPieces; i++) {
            const hash = await this.sha1Buffer(
                combinedBuffer.slice(i * pieceLength, Math.min((i + 1) * pieceLength, combinedBuffer.length))
            );
            piecesArray.set(hash, i * 20);
        }
        const info = {
            name: options.name || 'UnnamedTorrent',
            'piece length': pieceLength,
            pieces: Buffer.from(piecesArray),
            length: totalSize
        };
        if (fileInfos.length > 1) {
            info.files = fileInfos;
            delete info.length;
        } else if (fileInfos.length === 1) {
            info.length = fileInfos[0].length;
        }
        const torrent = {
            info,
            announce: this.options.trackerUrl,
            'creation date': Math.floor(Date.now() / 1000),
            'created by': 'Torrust-Actix v4.2',
            _originalFiles: allFileBuffers,
            _pieceLength: pieceLength
        };
        const bencodedInfo = bencoder.encode(torrent.info);
        const infoHash = await this.sha1Hex(bencodedInfo);
        const exportable = {
            info: torrent.info,
            announce: torrent.announce,
            'creation date': torrent['creation date'],
            'created by': torrent['created by'],
        };

        return {
            torrent,
            magnetUri: this.createMagnetURI(torrent, infoHash),
            infoHash,
            encodedTorrent: bencoder.encode(exportable)
        };
    }

    sha1Hex(data) {
        if (this.isBrowser) return this.sha1Browser(data);
        try {
            return require('crypto').createHash('sha1').update(data).digest('hex');
        } catch (_) {
            return '0'.repeat(40);
        }
    }

    async sha1Browser(data) {
        if (typeof data === 'string') data = new TextEncoder().encode(data);
        const buf = await crypto.subtle.digest('SHA-1', data);
        return Array.from(new Uint8Array(buf)).map(b => b.toString(16).padStart(2, '0')).join('');
    }

    async sha1Buffer(data) {
        if (this.isBrowser) {
            const buf = await crypto.subtle.digest('SHA-1', data);
            return new Uint8Array(buf);
        }
        try {
            return require('crypto').createHash('sha1').update(data).digest();
        } catch (_) {
            return new Uint8Array(20).fill(0x42);
        }
    }

    createMagnetURI(torrent, infoHash) {
        const trackers = Array.isArray(torrent.announce) ? torrent.announce : [torrent.announce];
        const params = [`xt=urn:btih:${infoHash}`];
        if (torrent.info?.name) params.push(`dn=${encodeURIComponent(torrent.info.name)}`);
        trackers.forEach(tr => tr && params.push(`tr=${encodeURIComponent(tr)}`));
        return `magnet:?${params.join('&')}`;
    }

    async calculateInfoHash(info) {
        const bencoder = this.createMinimalBencoder();
        return await this.sha1Hex(bencoder.encode(info));
    }

    async download(torrentData) {
        let torrent = torrentData;
        if (typeof torrentData === 'string') {
            torrent = torrentData.startsWith('magnet:')
                ? await this.parseMagnet(torrentData)
                : await this.fetchTorrentFile(torrentData);
        } else if (Buffer.isBuffer(torrentData) || torrentData instanceof Uint8Array) {
            torrent = await this.parseTorrentFile(torrentData);
        }
        const id = torrent.infoHash || await this.calculateInfoHash(torrent.info);
        if (this.torrents.has(id)) throw new Error('Torrent already exists');
        const inst = new Torrent(this, torrent, this.options);
        this.torrents.set(id, inst);
        return await inst.start();
    }

    /**
     * Seed a torrent.
     * @param {Buffer|Uint8Array|string|object} torrentData
     * @param {string[]|null} files
     */
    async seed(torrentData, files = null) {
        let torrent;
        if (typeof torrentData === 'string') {
            torrent = torrentData.startsWith('magnet:')
                ? await this.parseMagnet(torrentData)
                : await this.fetchTorrentFile(torrentData);
        } else if (Buffer.isBuffer(torrentData) || torrentData instanceof Uint8Array) {
            torrent = await this.parseTorrentFile(torrentData);
        } else {
            torrent = torrentData;
        }
        if (!torrent.info && torrent.torrent?.info) {
            torrent = { ...torrent, info: torrent.torrent.info };
        }
        if (files !== null && !this.isBrowser && this.fs) {
            const fileMeta = files
                .map(f => {
                    const p = typeof f === 'string' ? f : f.path;
                    if (!this.fs.existsSync(p)) return null;
                    return { path: p, size: this.fs.statSync(p).size };
                })
                .filter(Boolean);
            if (fileMeta.length > 0) {
                if (!torrent.torrent) torrent.torrent = {};
                torrent.torrent._filePaths   = fileMeta;
                torrent.torrent._pieceLength = torrent.info?.['piece length'] || 256 * 1024;
            }
        }
        if (files !== null && this.isBrowser && this.isFileApiAvailable) {
            const fileMeta = [];
            for (const f of files) {
                if (f instanceof File) fileMeta.push({ file: f, size: f.size });
            }
            if (fileMeta.length > 0) {
                if (!torrent.torrent) torrent.torrent = {};
                torrent.torrent._fileObjects  = fileMeta;
                torrent.torrent._pieceLength  = torrent.info?.['piece length'] || 256 * 1024;
            }
        }
        const id = torrent.infoHash || await this.calculateInfoHash(torrent.info);
        if (this.torrents.has(id)) throw new Error('Torrent already exists');
        const inst = new Torrent(this, torrent, this.options);
        this.torrents.set(id, inst);
        return await inst.seed();
    }

    async parseTorrentFile(torrentData) {
        const bencoder = await this.initBencoder();
        let buf;
        if (Buffer.isBuffer(torrentData)) buf = torrentData;
        else if (torrentData instanceof Uint8Array) buf = Buffer.from(torrentData);
        else throw new Error('Invalid torrent data type');
        const decoded = bencoder.decode(buf);
        const info = decoded.info;
        const bencodedInfo = bencoder.encode(info);
        const infoHash = await this.sha1Hex(bencodedInfo);
        const name = Buffer.isBuffer(info?.name) ? info.name.toString() : (info?.name || 'Unknown');
        return { info, announce: decoded.announce, infoHash, name };
    }

    async fetchTorrentFile(url) {
        const r = await _fetch(url);
        if (!r.ok) throw new Error(`Failed to fetch torrent: ${r.status}`);
        return this.parseTorrentFile(Buffer.from(await r.arrayBuffer()));
    }

    async parseMagnet(magnetUri) {
        const url = new URL(magnetUri);
        if (url.protocol !== 'magnet:') throw new Error('Invalid magnet URI');
        const params = Object.fromEntries(url.searchParams);
        const infoHash = (params.xt || '').replace('urn:btih:', '').toLowerCase();
        return {
            infoHash,
            announce: params.tr
                ? (Array.isArray(params.tr) ? params.tr : [params.tr])
                : [this.options.trackerUrl],
            name: params.dn ? decodeURIComponent(params.dn) : 'Unknown'
        };
    }

    async start() {
        this.webRtcManager = new WebRTCManager(this.options);
        this.announcer = new Announcer(this.options);
        console.log('RtcTorrent client started');
    }

    async stop() {
        for (const [, torrent] of this.torrents) await torrent.stop();
        if (this.webRtcManager) await this.webRtcManager.close();
        console.log('RtcTorrent client stopped');
    }

    async streamVideo(infoHash, fileIndex = 0, videoElement) {
        const torrent = this.torrents.get(infoHash);
        if (!torrent) throw new Error(`Torrent ${infoHash} not found`);
        return torrent.streamVideo(fileIndex, videoElement);
    }
}

class Torrent {
    constructor(client, torrentData, options) {
        this.client = client;
        this.data = torrentData;
        this.options = options;
        this.peers = new Map();
        this.pieces = new Map();
        this.downloaded = 0;
        this.uploaded = 0;
        this.active = false;
        this.isSeeder = false;
        this.signalChannel = null;
        this.pieceLength = torrentData.info?.['piece length'] || 256 * 1024;
        this.totalSize = this.calculateTotalSize(torrentData);
        this.files = this.extractFiles(torrentData);
        this.pieceCount = Math.ceil(this.totalSize / this.pieceLength) || 0;
        this._peerIdBytes = this._generatePeerIdBytes();
        this._peerIdHex = Buffer.from(this._peerIdBytes).toString('hex');
        this._localPc = null;
        this._localSdp = null;
        this.mediaSource = null;
        this.sourceBuffer = null;
        this.requestedPieces = new Set();
        this._downloadCompleted = false;
        this._sendQueue = Promise.resolve();
    }

    calculateTotalSize(torrentData) {
        if (torrentData.info?.length) return torrentData.info.length;
        if (Array.isArray(torrentData.info?.files)) {
            return torrentData.info.files.reduce((s, f) => s + (f.length || 0), 0);
        }
        return 0;
    }

    extractFiles(torrentData) {
        if (torrentData.info?.length && torrentData.info?.name) {
            const name = Buffer.isBuffer(torrentData.info.name)
                ? torrentData.info.name.toString()
                : torrentData.info.name;
            return [{ name, length: torrentData.info.length, offset: 0 }];
        }
        if (Array.isArray(torrentData.info?.files)) {
            let offset = 0;
            return torrentData.info.files.map(f => {
                const fi = {
                    name: Array.isArray(f.path)
                        ? f.path.map(p => Buffer.isBuffer(p) ? p.toString() : p).join('/')
                        : (Buffer.isBuffer(f.path) ? f.path.toString() : f.path),
                    length: f.length,
                    offset
                };
                offset += f.length;
                return fi;
            });
        }
        return [];
    }

    _generatePeerIdBytes() {
        const prefix = '-RT1000-';
        let id = prefix;
        for (let i = 0; i < 20 - prefix.length; i++) {
            id += Math.floor(Math.random() * 10).toString();
        }
        return Buffer.from(id, 'ascii');
    }

    urlEncodeBytes(bytes) {
        let out = '';
        for (let i = 0; i < bytes.length; i++) {
            out += '%' + bytes[i].toString(16).padStart(2, '0');
        }
        return out;
    }

    hexToUint8Array(hex) {
        const b = new Uint8Array(hex.length / 2);
        for (let i = 0; i < b.length; i++) b[i] = parseInt(hex.substr(i * 2, 2), 16);
        return b;
    }

    async start() {
        this.active = true;
        this.signalChannel = new SignalChannel(this.client.options.trackerUrl, this.data.infoHash);
        this.signalChannel.on('offer',  sdp => this.handleOffer(sdp));
        this.signalChannel.on('answer', sdp => this.handleAnswer(sdp));
        this.announceLoop();
        console.log(`Started torrent ${this.data.infoHash}`);
        return this;
    }

    async seed() {
        this.isSeeder   = true;
        this.downloaded = this.totalSize;
        return this.start();
    }

    async stop() {
        this.active = false;
        for (const [, peer] of this.peers) {
            try { peer.connection?.close(); } catch (_) {}
        }
        if (this.signalChannel) this.signalChannel.close();
        if (this._localPc) { try { this._localPc.close(); } catch (_) {} }
        console.log(`Stopped torrent ${this.data.infoHash}`);
    }

    async announceLoop() {
        if (!this.active) return;
        try {
            const isSeeder = this.isSeeder || (this.totalSize > 0 && this.downloaded >= this.totalSize);
            let rtcResponse;
            if (isSeeder) {
                const sdpOffer = await this.getOrCreateSdpOffer();
                rtcResponse = await this.announce(
                    { event: 'started', numwant: 50 },
                    { rtctorrent: true, rtcoffer: sdpOffer }
                );
                if (rtcResponse?.rtc_answers?.length) {
                    for (const answer of rtcResponse.rtc_answers) {
                        await this.handleAnswerFromTracker(answer);
                    }
                }
            } else {
                rtcResponse = await this.announce(
                    { event: 'started', numwant: 50 },
                    { rtctorrent: true, rtcrequest: true }
                );
                if (rtcResponse?.rtc_peers?.length) {
                    for (const peer of rtcResponse.rtc_peers) {
                        if (!this.peers.has(peer.peer_id)) {
                            await this.connectToWebRTCPeer(peer);
                        }
                    }
                }
                if (!this.isSeeder) this._requestMissingPieces();
            }
            if (rtcResponse?.rtc_interval) {
                this.client.options.rtcInterval = rtcResponse.rtc_interval;
            }
        } catch (err) {
            console.error('RtcTorrent announce error:', err);
        } finally {
            if (this.active) {
                setTimeout(() => this.announceLoop(), this.client.options.rtcInterval);
            }
        }
    }

    _requestMissingPieces() {
        const toRequest = Math.max(0, MAX_IN_FLIGHT - this.requestedPieces.size);
        if (toRequest === 0) return;
        let requested = 0;
        for (let i = 0; i < this.pieceCount && requested < toRequest; i++) {
            if (!this.pieces.has(i) && !this.requestedPieces.has(i)) {
                this.requestPieceFromPeers(i);
                this.requestedPieces.add(i);
                requested++;
            }
        }
    }

    /**
     * Make a tracker announce request.
     * @param {object} params
     * @param {object|null} rtcParams
     */
    async announce(params = {}, rtcParams = null) {
        const infoHashBytes = this.hexToUint8Array(this.data.infoHash);
        const left = Math.max(0, this.totalSize - this.downloaded);
        const parts = [
            'info_hash=' + this.urlEncodeBytes(infoHashBytes),
            'peer_id=' + this.urlEncodeBytes(this._peerIdBytes),
            'port=6881',
            'uploaded=' + this.uploaded,
            'downloaded=' + this.downloaded,
            'left=' + left,
            'compact=1'
        ];
        if (params.event) parts.push('event=' + params.event);
        if (params.numwant) parts.push('numwant=' + params.numwant);
        if (rtcParams) {
            parts.push('rtctorrent=1');
            if (rtcParams.rtcoffer) parts.push('rtcoffer=' + encodeURIComponent(rtcParams.rtcoffer));
            if (rtcParams.rtcrequest) parts.push('rtcrequest=1');
            if (rtcParams.rtcanswer) parts.push('rtcanswer=' + encodeURIComponent(rtcParams.rtcanswer));
            if (rtcParams.rtcanswerfor) parts.push('rtcanswerfor=' + encodeURIComponent(rtcParams.rtcanswerfor));
        }
        const url = `${this.client.options.trackerUrl}?${parts.join('&')}`;
        try {
            const res = await _fetch(url);
            if (!res.ok) throw new Error(`Tracker returned ${res.status}`);
            const ab = await res.arrayBuffer();
            return this.parseBencodedResponse(new Uint8Array(ab));
        } catch (err) {
            console.error('Announce failed:', err.message);
            throw err;
        }
    }

    parseBencodedResponse(responseBytes) {
        try {
            let bencode;
            try { bencode = require('bencode'); } catch (_) { bencode = null; }
            if (!bencode) {
                console.warn('bencode library not available – tracker response may not be fully parsed');
                return {};
            }
            const decoded = bencode.decode(Buffer.from(responseBytes));
            const str = (v) => Buffer.isBuffer(v) ? v.toString() : (v != null ? String(v) : null);
            const num = (v) => v != null ? Number(v) : undefined;
            const result = {};
            if (decoded.interval) result.interval = num(decoded.interval);
            if (decoded['rtc interval']) result.rtc_interval = num(decoded['rtc interval']) * 1000;
            if (decoded.complete != null) result.complete = num(decoded.complete);
            if (decoded.incomplete != null) result.incomplete = num(decoded.incomplete);
            if (Array.isArray(decoded.rtc_peers)) {
                result.rtc_peers = decoded.rtc_peers
                    .map(p => ({
                        peer_id: Buffer.isBuffer(p.peer_id) ? p.peer_id.toString('hex') : str(p.peer_id),
                        sdp_offer: str(p.sdp_offer)
                    }))
                    .filter(p => p.peer_id && p.sdp_offer);
            }
            if (Array.isArray(decoded.rtc_answers)) {
                result.rtc_answers = decoded.rtc_answers
                    .map(a => ({
                        peer_id: Buffer.isBuffer(a.peer_id) ? a.peer_id.toString('hex') : str(a.peer_id),
                        sdp_answer: str(a.sdp_answer)
                    }))
                    .filter(a => a.peer_id && a.sdp_answer);
            }
            if (decoded['failure reason']) {
                result.failure_reason = str(decoded['failure reason']);
                console.error('Tracker failure:', result.failure_reason);
            }
            return result;
        } catch (err) {
            console.error('Error parsing bencoded response:', err.message);
            return {};
        }
    }

    async getOrCreateSdpOffer() {
        if (this._localSdp && this._localPc &&
            this._localPc.signalingState !== 'closed' &&
            this._localPc.connectionState !== 'connected') {
            return this._localSdp;
        }
        const RTCPeerConnection = this.client.getRTCPeerConnection();
        if (!RTCPeerConnection) {
            console.warn('RTCPeerConnection not available. ' +
                'In Node.js, install "wrtc" or "@roamhq/wrtc" for WebRTC support.');
            return null;
        }
        try {
            const pc = new RTCPeerConnection({ iceServers: this.client.options.iceServers });
            const dc = pc.createDataChannel('torrent', { ordered: false, maxRetransmits: 3 });
            this._setupDataChannelSeeder(dc);
            const offer = await pc.createOffer({ offerToReceiveAudio: false, offerToReceiveVideo: false });
            await pc.setLocalDescription(offer);
            await this._waitForIceGathering(pc);
            this._localPc = pc;
            this._localSdp = pc.localDescription.sdp;
            console.log(`[Seeder] SDP offer created (${this._localSdp.length} bytes)`);
            return this._localSdp;
        } catch (err) {
            console.error('Failed to create SDP offer:', err);
            return null;
        }
    }

    _waitForIceGathering(pc) {
        return new Promise((resolve) => {
            if (pc.iceGatheringState === 'complete') { resolve(); return; }
            const done = () => { if (pc.iceGatheringState === 'complete') resolve(); };
            pc.onicegatheringstatechange = done;
            setTimeout(resolve, 5000);
        });
    }

    _setupDataChannelSeeder(dc) {
        dc.onopen = () => {
            console.log('[Seeder] Data channel opened');
        };
        dc.onerror = (e) => console.error('[Seeder] Data channel error:', e);
        dc.onclose = () => console.log('[Seeder] Data channel closed');
        dc.onmessage = (event) => this.handleMessage(event.data, null, dc);
    }

    async connectToWebRTCPeer(peerInfo) {
        if (!peerInfo?.sdp_offer) {
            console.warn('connectToWebRTCPeer: peerInfo.sdp_offer missing');
            return;
        }
        const RTCPeerConnection = this.client.getRTCPeerConnection();
        if (!RTCPeerConnection) {
            console.warn('RTCPeerConnection not available – skipping peer connection');
            return;
        }
        try {
            const pc = new RTCPeerConnection({ iceServers: this.client.options.iceServers });
            pc.ondatachannel = (event) => {
                const dc = event.channel;
                this.setupDataChannel(dc, peerInfo);
                const existing = this.peers.get(peerInfo.peer_id);
                if (existing) existing.channel = dc;
                if (dc.readyState === 'open') {
                    this.requestedPieces.clear();
                    this._requestMissingPieces();
                }
            };
            await pc.setRemoteDescription({ type: 'offer', sdp: peerInfo.sdp_offer });
            const answer = await pc.createAnswer();
            await pc.setLocalDescription(answer);
            await this._waitForIceGathering(pc);
            const answerSdp = pc.localDescription.sdp;
            this.peers.set(peerInfo.peer_id, {
                connection: pc,
                channel: null,
                info: peerInfo,
                connected: false
            });
            await this.announce(
                { event: 'started', numwant: 0 },
                {
                    rtctorrent: true,
                    rtcanswer: answerSdp,
                    rtcanswerfor: peerInfo.peer_id
                }
            );
            console.log(`[Leecher] Sent SDP answer to seeder ${peerInfo.peer_id}`);
        } catch (err) {
            console.error('Error connecting to WebRTC peer:', err);
        }
    }

    async handleAnswerFromTracker(answerInfo) {
        if (!this._localPc || !answerInfo?.sdp_answer) return;
        try {
            if (this._localPc.signalingState !== 'have-local-offer') {
                console.warn('[Seeder] Cannot apply answer: signalingState =', this._localPc.signalingState);
                return;
            }
            await this._localPc.setRemoteDescription({ type: 'answer', sdp: answerInfo.sdp_answer });
            console.log(`[Seeder] WebRTC connection established with leecher ${answerInfo.peer_id}`);
            this.peers.set(answerInfo.peer_id, {
                connection: this._localPc,
                channel: null,
                info: { peer_id: answerInfo.peer_id },
                connected: false
            });
            this._localPc = null;
            this._localSdp = null;
        } catch (err) {
            console.error('[Seeder] Error applying SDP answer:', err);
        }
    }

    setupDataChannel(channel, peerInfo) {
        channel.binaryType = 'arraybuffer';
        channel.onopen = () => {
            console.log('[Leecher] Data channel opened with peer', peerInfo?.peer_id);
            if (peerInfo?.peer_id && this.peers.has(peerInfo.peer_id)) {
                this.peers.get(peerInfo.peer_id).connected = true;
            }
            this.requestedPieces.clear();
            this._requestMissingPieces();
        };
        channel.onclose = () => console.log('[Leecher] Data channel closed');
        channel.onerror = (e) => console.error('[Leecher] Data channel error:', e);
        channel.onmessage = (event) => this.handleMessage(event.data, peerInfo, channel);
    }

    handleMessage(data, peerInfo, channel) {
        try {
            let view;
            if (data instanceof ArrayBuffer) {
                view = new DataView(data);
            } else if (ArrayBuffer.isView(data)) {
                view = new DataView(data.buffer, data.byteOffset, data.byteLength);
            } else {
                return;
            }
            const msgType = view.getUint8(0);
            if (msgType === MSG_PIECE_REQUEST) {
                const pieceIndex = view.getUint32(1, false);
                this.handlePieceRequest(pieceIndex, channel);
            } else if (msgType === MSG_PIECE_DATA) {
                const pieceIndex = view.getUint32(1, false);
                const pieceData = data instanceof ArrayBuffer
                    ? new Uint8Array(data, 5)
                    : data.subarray(5);
                this.handlePieceData(pieceIndex, pieceData, peerInfo);
            }
        } catch (err) {
            console.error('Error handling message:', err);
        }
    }

    handlePieceRequest(pieceIndex, channel) {
        this._sendQueue = this._sendQueue.then(() => this._servePiece(pieceIndex, channel));
    }

    async _servePiece(pieceIndex, channel) {
        if (!channel || channel.readyState !== 'open') return;
        try {
            const pieceData = await this._readPieceAsync(pieceIndex);
            if (!pieceData || pieceData.length === 0) return;
            if (channel.bufferedAmount > 512 * 1024) {
                await new Promise(resolve => {
                    if (channel.readyState !== 'open') { resolve(); return; }
                    channel.bufferedAmountLowThreshold = 64 * 1024;
                    channel.addEventListener('bufferedamountlow', resolve, { once: true });
                    setTimeout(resolve, 10000);
                });
            }
            if (channel.readyState !== 'open') return;
            const frame = new Uint8Array(5 + pieceData.length);
            frame[0] = MSG_PIECE_DATA;
            new DataView(frame.buffer).setUint32(1, pieceIndex, false);
            frame.set(pieceData, 5);
            channel.send(frame.buffer);
            console.log(`[Seeder] Sent piece ${pieceIndex} (${pieceData.length} bytes)`);
        } catch (err) {
            console.error('[Seeder] Error serving piece:', err);
        }
    }

    _readPiece(pieceIndex) {
        const pl = this.data.torrent?._pieceLength || this.pieceLength;
        const start = pieceIndex * pl;
        const end   = Math.min(start + pl, this.totalSize);
        if (start >= this.totalSize) return null;
        const filePaths = this.data.torrent?._filePaths;
        if (filePaths && this.client.fs) {
            return this._readRangeFromDisk(filePaths, start, end);
        }
        const buffers = this.data.torrent?._originalFiles;
        if (buffers) {
            const combined = Buffer.concat(buffers);
            return combined.slice(start, Math.min(end, combined.length));
        }
        return null;
    }

    _readRangeFromDisk(filePaths, start, end) {
        const fs = this.client.fs;
        const result = Buffer.alloc(end - start);
        let resultOffset = 0;
        let fileStart = 0;
        for (const fileMeta of filePaths) {
            const fileEnd = fileStart + fileMeta.size;
            if (fileEnd <= start)  { fileStart = fileEnd; continue; }
            if (fileStart >= end)  break;
            const readFrom = Math.max(start, fileStart) - fileStart;
            const readTo = Math.min(end, fileEnd) - fileStart;
            const len = readTo - readFrom;
            const fd = fs.openSync(fileMeta.path, 'r');
            try {
                fs.readSync(fd, result, resultOffset, len, readFrom);
            } finally {
                fs.closeSync(fd);
            }
            resultOffset += len;
            fileStart = fileEnd;
        }
        return result;
    }

    async _readPieceAsync(pieceIndex) {
        const pl = this.data.torrent?._pieceLength || this.pieceLength;
        const start = pieceIndex * pl;
        const end = Math.min(start + pl, this.totalSize);
        if (start >= this.totalSize) return null;
        const fileObjects = this.data.torrent?._fileObjects;
        if (fileObjects) {
            return await this._readRangeFromFileObjects(fileObjects, start, end);
        }
        return this._readPiece(pieceIndex);
    }

    async _readRangeFromFileObjects(fileObjects, start, end) {
        const result = new Uint8Array(end - start);
        let resultOffset = 0;
        let fileStart = 0;
        for (const fm of fileObjects) {
            const fileEnd = fileStart + fm.size;
            if (fileEnd <= start) { fileStart = fileEnd; continue; }
            if (fileStart >= end) break;
            const readFrom = Math.max(start, fileStart) - fileStart;
            const readTo = Math.min(end, fileEnd) - fileStart;
            const len = readTo - readFrom;
            const ab = await fm.file.slice(readFrom, readTo).arrayBuffer();
            result.set(new Uint8Array(ab), resultOffset);
            resultOffset += len;
            fileStart = fileEnd;
        }
        return result;
    }

    handlePieceData(pieceIndex, pieceData, peerInfo) {
        if (this.pieces.has(pieceIndex)) return;
        this._verifyPiece(pieceIndex, pieceData).then(ok => {
            if (!ok) {
                console.warn(`[Leecher] Piece ${pieceIndex} hash mismatch — discarding, will re-request`);
                this.requestedPieces.delete(pieceIndex);
                return;
            }
            this.pieces.set(pieceIndex, pieceData);
            this.requestedPieces.delete(pieceIndex);
            const added = Math.min(pieceData.length, this.totalSize - this.downloaded);
            this.downloaded += added;
            console.log(`[Leecher] Piece ${pieceIndex} OK (${pieceData.length} B) — ` +
                `${this.pieces.size}/${this.pieceCount} pieces`);
            if (typeof this.onPieceReceived === 'function') {
                this.onPieceReceived(pieceIndex, pieceData);
            }
            this.updateProgress();
            if (!this._downloadCompleted) this._requestMissingPieces();
        });
    }

    async _verifyPiece(pieceIndex, pieceData) {
        const piecesHash = this.data.info?.pieces;
        if (!piecesHash) return true;
        const expected = piecesHash.slice(pieceIndex * 20, pieceIndex * 20 + 20);
        if (!expected || expected.length < 20) return true;
        let actual;
        try {
            if (typeof window !== 'undefined' && window.crypto?.subtle) {
                const buf = pieceData instanceof Uint8Array
                    ? pieceData.buffer.slice(pieceData.byteOffset, pieceData.byteOffset + pieceData.byteLength)
                    : pieceData;
                const hash = await window.crypto.subtle.digest('SHA-1', buf);
                actual = new Uint8Array(hash);
            } else {
                const hash = require('crypto').createHash('sha1')
                    .update(pieceData instanceof Buffer ? pieceData : Buffer.from(pieceData))
                    .digest();
                actual = new Uint8Array(hash);
            }
        } catch (e) {
            console.warn('[RtcTorrent] SHA-1 verify failed:', e.message);
            return true;
        }
        for (let i = 0; i < 20; i++) {
            if (actual[i] !== (Buffer.isBuffer(expected) ? expected[i] : expected[i])) return false;
        }
        return true;
    }

    async handleOffer(sdpOffer) {
        const RTCPeerConnection = this.client.getRTCPeerConnection();
        if (!RTCPeerConnection) return;
        const pc = new RTCPeerConnection({ iceServers: this.client.options.iceServers });
        pc.ondatachannel = (e) => this.setupDataChannel(e.channel, null);
        await pc.setRemoteDescription({ type: 'offer', sdp: sdpOffer });
        const answer = await pc.createAnswer();
        await pc.setLocalDescription(answer);
        this.signalChannel.sendAnswer(answer.sdp);
        this.peers.set(`incoming_${Date.now()}`, { connection: pc, channel: null, info: {}, connected: false });
    }

    async handleAnswer(sdpAnswer) {
        for (const [, peer] of this.peers) {
            if (!peer.connected && peer.connection?.signalingState === 'have-local-offer') {
                try {
                    await peer.connection.setRemoteDescription({ type: 'answer', sdp: sdpAnswer });
                    peer.connected = true;
                    break;
                } catch (err) {
                    console.error('Error setting remote answer:', err);
                }
            }
        }
    }

    async requestPieceFromPeers(pieceIndex) {
        const frame = new Uint8Array(5);
        frame[0] = MSG_PIECE_REQUEST;
        new DataView(frame.buffer).setUint32(1, pieceIndex, false);
        let sent = false;
        for (const [, peer] of this.peers) {
            if (peer.channel?.readyState === 'open') {
                try {
                    peer.channel.send(frame.buffer);
                    sent = true;
                    break;
                } catch (_) {}
            }
        }
        if (!sent) {
            console.log(`[Leecher] No open channel to request piece ${pieceIndex}`);
        }
    }

    updateProgress() {
        if (!this._downloadCompleted && this.pieceCount > 0 && this.pieces.size >= this.pieceCount) {
            this._downloadCompleted = true;
            this.onDownloadComplete();
        }
    }

    onDownloadComplete() {
        console.log('[Leecher] Download complete!');
    }

    /**
     * Play a file from this torrent in a <video> element.
     *
     * Strategy:
     *  1. If the browser supports MediaSource AND the file starts with an
     *     MP4 'ftyp' box (i.e. faststart / fragmented MP4), stream pieces
     *     directly into a SourceBuffer as they arrive — true progressive
     *     streaming, playback starts immediately.
     *  2. Otherwise wait for the full download and create a Blob URL.
     *     A progress callback is invoked while waiting.
     *
     * @param {number}      fileIndex
     * @param {HTMLVideoElement} videoElement
     * @param {object}      opts
     * @param {string}      opts.mimeType
     * @param {function}    opts.onProgress
     */
    async streamVideo(fileIndex = 0, videoElement, opts = {}) {
        if (!videoElement) throw new Error('videoElement is required');
        const file = this.files[fileIndex];
        if (!file) throw new Error(`File at index ${fileIndex} not found`);
        if (typeof window === 'undefined') throw new Error('streamVideo is browser-only');
        const firstPiece = Math.floor(file.offset / this.pieceLength);
        await this._waitForPiece(firstPiece);
        const detectedMime = opts.mimeType || this._detectMimeType(this.pieces.get(firstPiece));
        if (detectedMime && window.MediaSource && MediaSource.isTypeSupported(detectedMime)) {
            try {
                await this._streamWithMSE(file, videoElement, detectedMime);
                return;
            } catch (e) {
                console.warn('[RtcTorrent] MSE streaming failed, falling back to Blob URL:', e.message);
                if (videoElement.src?.startsWith('blob:')) URL.revokeObjectURL(videoElement.src);
            }
        }
        const moovEndByte = this._getFaststartMoovEnd(this.pieces.get(firstPiece));
        if (moovEndByte > 0) {
            console.log(`[RtcTorrent] Faststart MP4 detected (moov ends at byte ${moovEndByte}) — progressive blob streaming`);
            this._moovEndByte = moovEndByte;
            await this._streamFaststart(file, videoElement, opts.onProgress);
            return;
        }
        await this._playAsBlob(file, videoElement, opts.onProgress);
    }

    _detectMimeType(piece0) {
        if (!piece0 || piece0.length < 12) return null;
        const bytes = piece0 instanceof Uint8Array ? piece0 : new Uint8Array(piece0);
        if (String.fromCharCode(bytes[4], bytes[5], bytes[6], bytes[7]) !== 'ftyp') return null;
        const ftypSize = (bytes[0] << 24 | bytes[1] << 16 | bytes[2] << 8 | bytes[3]) >>> 0;
        if (ftypSize < 16) return null;
        const FRAG_BRANDS = new Set(['iso5', 'iso6', 'cmfc', 'cmff', 'dash']);
        const major = String.fromCharCode(bytes[8], bytes[9], bytes[10], bytes[11]);
        if (FRAG_BRANDS.has(major)) return 'video/mp4; codecs="avc1.42E01E,mp4a.40.2"';
        const end = Math.min(ftypSize, bytes.length);
        for (let off = 16; off + 4 <= end; off += 4) {
            const brand = String.fromCharCode(bytes[off], bytes[off+1], bytes[off+2], bytes[off+3]);
            if (FRAG_BRANDS.has(brand)) return 'video/mp4; codecs="avc1.42E01E,mp4a.40.2"';
        }
        return null;
    }

    _getFaststartMoovEnd(piece0) {
        if (!piece0 || piece0.length < 16) return 0;
        const b = piece0 instanceof Uint8Array ? piece0 : new Uint8Array(piece0);
        if (String.fromCharCode(b[4], b[5], b[6], b[7]) !== 'ftyp') return 0;
        const ftypSize = (b[0] << 24 | b[1] << 16 | b[2] << 8 | b[3]) >>> 0;
        if (ftypSize < 8 || ftypSize + 8 > b.length) return 0;
        let off = ftypSize;
        while (off + 8 <= b.length) {
            const boxSize = (b[off] << 24 | b[off+1] << 16 | b[off+2] << 8 | b[off+3]) >>> 0;
            const boxType = String.fromCharCode(b[off+4], b[off+5], b[off+6], b[off+7]);
            if (boxSize < 8) return 0;
            if (boxType === 'moov') {
                return off + boxSize;
            }
            if (boxType === 'mdat') {
                return 0;
            }
            off += boxSize;
        }
        return 0;
    }

    async _streamFaststart(file, videoElement, onProgress) {
        const INITIAL_EXTRA = 8;
        const BUFFER_AHEAD_S = 20;
        const CHECK_INTERVAL = 2000;
        const startPiece = Math.floor(file.offset / this.pieceLength);
        const endPiece = Math.ceil((file.offset + file.length) / this.pieceLength);
        const totalPieces = endPiece - startPiece;
        const moovEndByte  = this._moovEndByte || 0;
        const moovEndPiece = Math.max(
            startPiece,
            Math.floor((file.offset + moovEndByte - 1) / this.pieceLength)
        );
        const initialEndPiece = Math.min(moovEndPiece + INITIAL_EXTRA, endPiece - 1);
        console.log(
            `[RtcTorrent] faststart stream: moovEndByte=${moovEndByte}` +
            ` moovEndPiece=${moovEndPiece} initialEndPiece=${initialEndPiece}` +
            ` totalPieces=${totalPieces}`
        );
        for (let i = startPiece; i <= initialEndPiece; i++) {
            await this._waitForPiece(i);
            if (typeof onProgress === 'function') {
                onProgress(Math.floor((i - startPiece + 1) / totalPieces * 100));
            }
        }
        let blobUrl = null;
        let lastBlobPiece = -1;
        let buildInProgress = false;
        let pendingBuildTo = -1;
        let firstPlayDone = false;
        const getContiguousEnd = () => {
            let latest = (lastBlobPiece >= startPiece) ? lastBlobPiece : startPiece - 1;
            while (latest + 1 < endPiece && this.pieces.has(latest + 1)) latest++;
            return latest;
        };
        const buildAndApplyBlob = async (upToPiece) => {
            if (upToPiece <= lastBlobPiece) return;
            if (buildInProgress) {
                pendingBuildTo = Math.max(pendingBuildTo, upToPiece);
                return;
            }
            buildInProgress = true;
            try {
                const chunks = [];
                const startOffset = file.offset % this.pieceLength;
                let bytesSoFar = 0;
                for (let i = startPiece; i <= upToPiece; i++) {
                    const p = this.pieces.get(i);
                    if (!p) break;
                    const data = p instanceof Uint8Array ? p : new Uint8Array(p);
                    const from = (i === startPiece) ? startOffset : 0;
                    let to = data.length;
                    if (i === endPiece - 1) to = from + (file.length - bytesSoFar);
                    chunks.push(data.slice(from, to));
                    bytesSoFar += (to - from);
                    lastBlobPiece = i;
                }
                if (chunks.length === 0) return;
                const prevTime   = isNaN(videoElement.currentTime) ? 0 : videoElement.currentTime;
                const shouldPlay = !videoElement.paused
                    || videoElement.readyState < 3
                    || videoElement.ended;
                if (blobUrl) URL.revokeObjectURL(blobUrl);
                const mimeType = this._getMimeForFile(file) || 'video/mp4';
                const blob = new Blob(chunks, { type: mimeType });
                blobUrl = URL.createObjectURL(blob);
                videoElement.src = blobUrl;
                videoElement.load();
                await new Promise(resolve => {
                    const onMeta = () => {
                        videoElement.removeEventListener('loadedmetadata', onMeta);
                        const target = Math.min(prevTime, videoElement.duration || 0);
                        if (target > 0.5) videoElement.currentTime = target;
                        resolve();
                    };
                    videoElement.addEventListener('loadedmetadata', onMeta, { once: true });
                    setTimeout(resolve, 2000);
                });
                if (!firstPlayDone || shouldPlay) {
                    firstPlayDone = true;
                    videoElement.play().catch(e => {
                        console.warn('[RtcTorrent] Faststart autoplay blocked (click play):', e.message);
                    });
                }
                console.log(`[RtcTorrent] blob extended to piece ${lastBlobPiece}/${endPiece - 1}` +
                    ` (${(blob.size / 1024 / 1024).toFixed(1)} MB)`);
            } finally {
                buildInProgress = false;
                if (pendingBuildTo > lastBlobPiece) {
                    const next = pendingBuildTo;
                    pendingBuildTo = -1;
                    setTimeout(() => buildAndApplyBlob(next), 0);
                }
            }
        };
        const origOnPieceReceived = this.onPieceReceived;
        const cleanup = (checkIntervalRef) => {
            clearInterval(checkIntervalRef);
            videoElement.removeEventListener('waiting', onVideoStall);
            videoElement.removeEventListener('ended',  onVideoStall);
            this.onPieceReceived = origOnPieceReceived;
        };
        await buildAndApplyBlob(initialEndPiece);
        const onVideoStall = () => {
            const latest = getContiguousEnd();
            if (latest > lastBlobPiece) buildAndApplyBlob(latest).catch(() => {});
        };
        videoElement.addEventListener('waiting', onVideoStall);
        videoElement.addEventListener('ended',   onVideoStall);
        this.onPieceReceived = (pieceIndex, pieceData) => {
            if (origOnPieceReceived) origOnPieceReceived.call(this, pieceIndex, pieceData);
            if (pieceIndex > lastBlobPiece) {
                const latest = getContiguousEnd();
                if (latest > lastBlobPiece) buildAndApplyBlob(latest).catch(() => {});
            }
        };
        const checkIntervalId = setInterval(async () => {
            const latest = getContiguousEnd();
            if (typeof onProgress === 'function') {
                const got = [...this.pieces.keys()].filter(k => k >= startPiece && k < endPiece).length;
                onProgress(Math.floor(got / totalPieces * 100));
            }
            if (this._downloadCompleted || latest >= endPiece - 1) {
                cleanup(checkIntervalId);
                await buildAndApplyBlob(endPiece - 1);
                console.log('[RtcTorrent] faststart stream complete — full blob playing');
                return;
            }
            let bufferedAhead = 0;
            try {
                const ct  = videoElement.currentTime;
                const buf = videoElement.buffered;
                for (let i = 0; i < buf.length; i++) {
                    if (buf.start(i) <= ct + 0.5 && buf.end(i) > ct) {
                        bufferedAhead = buf.end(i) - ct;
                        break;
                    }
                }
            } catch (_) {}
            if (bufferedAhead < BUFFER_AHEAD_S && latest > lastBlobPiece) {
                await buildAndApplyBlob(latest);
            }
        }, CHECK_INTERVAL);
    }

    async _waitForPiece(index) {
        while (!this.pieces.has(index)) {
            if (!this.requestedPieces.has(index)) {
                this.requestPieceFromPeers(index);
                this.requestedPieces.add(index);
            }
            await new Promise(r => setTimeout(r, 100));
        }
    }

    async _streamWithMSE(file, videoElement, mimeType) {
        const startPiece = Math.floor(file.offset / this.pieceLength);
        const endPiece   = Math.ceil((file.offset + file.length) / this.pieceLength);
        const ms = new MediaSource();
        this.mediaSource = ms;
        videoElement.src = URL.createObjectURL(ms);
        await new Promise((resolve, reject) => {
            ms.addEventListener('sourceopen', async () => {
                let sb;
                try {
                    sb = ms.addSourceBuffer(mimeType);
                    this.sourceBuffer = sb;
                    const raw = this.pieces.get(startPiece);
                    await this._appendToSourceBuffer(sb, raw);
                    resolve();
                } catch (e) {
                    reject(e);
                    return;
                }
                (async () => {
                    try {
                        for (let i = startPiece + 1; i < endPiece; i++) {
                            if (ms.readyState !== 'open') break;
                            await this._waitForPiece(i);
                            if (ms.readyState !== 'open') break;
                            await this._appendToSourceBuffer(sb, this.pieces.get(i));
                        }
                        if (ms.readyState === 'open') ms.endOfStream();
                    } catch (e) {
                        console.warn('[RtcTorrent] MSE feed error:', e.message);
                        try { if (ms.readyState === 'open') ms.endOfStream('decode'); } catch (_) {}
                    }
                })();
            }, { once: true });
        });
    }

    _appendToSourceBuffer(sb, data) {
        return new Promise((resolve, reject) => {
            const done  = () => { sb.removeEventListener('updateend', done);  sb.removeEventListener('error', fail); resolve(); };
            const fail  = (e) => { sb.removeEventListener('updateend', done); sb.removeEventListener('error', fail); reject(e); };
            sb.addEventListener('updateend', done);
            sb.addEventListener('error', fail);
            let buf;
            if (data instanceof ArrayBuffer) {
                buf = data;
            } else if (ArrayBuffer.isView(data)) {
                buf = data.buffer.slice(data.byteOffset, data.byteOffset + data.byteLength);
            } else {
                buf = data.buffer || data;
            }
            sb.appendBuffer(buf);
        });
    }

    async _playAsBlob(file, videoElement, onProgress) {
        const startPiece = Math.floor(file.offset / this.pieceLength);
        const endPiece = Math.ceil((file.offset + file.length) / this.pieceLength);
        const total = endPiece - startPiece;
        for (let i = startPiece; i < endPiece; i++) {
            await this._waitForPiece(i);
            if (typeof onProgress === 'function' && !this._downloadCompleted) {
                const pct = Math.floor((i - startPiece + 1) / total * 100);
                onProgress(pct);
            }
        }
        console.log('[RtcTorrent] Assembling Blob…');
        const blob = await this.getBlob(file);
        console.log(`[RtcTorrent] Blob ready (${(blob.size / 1024 / 1024).toFixed(1)} MB), setting video src`);
        const url = URL.createObjectURL(blob);
        videoElement.src = url;
        videoElement.load();
        const playPromise = videoElement.play();
        if (playPromise !== undefined) {
            playPromise.catch(e => {
                console.warn('[RtcTorrent] Autoplay blocked (click play to start):', e.message);
            });
        }
    }

    /**
     * Assemble all pieces for a file into a Blob.
     * @param {object|number} fileOrIndex
     * @returns {Blob}
     */
    async getBlob(fileOrIndex = 0) {
        const file = (typeof fileOrIndex === 'number') ? this.files[fileOrIndex] : fileOrIndex;
        if (!file) throw new Error('File not found');
        const startPiece = Math.floor(file.offset / this.pieceLength);
        const endPiece = Math.ceil((file.offset + file.length) / this.pieceLength);
        const startOff = file.offset % this.pieceLength;
        const chunks = [];
        for (let i = startPiece; i < endPiece; i++) {
            const raw = this.pieces.get(i);
            if (!raw) throw new Error(`Piece ${i} not available`);
            const from = (i === startPiece) ? startOff : 0;
            const to   = (i === endPiece - 1)
                ? from + (file.length - chunks.reduce((s, c) => s + c.byteLength, 0))
                : raw.length;
            chunks.push(raw instanceof Uint8Array
                ? raw.slice(from, to)
                : new Uint8Array(raw.buffer || raw, raw.byteOffset || 0, raw.byteLength || raw.length).slice(from, to));
        }
        return new Blob(chunks, { type: this._getMimeForFile(file) });
    }

    _getMimeForFile(file) {
        const ext = ((file.name || '').split('/').pop()?.split('.').pop() || '').toLowerCase();
        const map = {
            mp4: 'video/mp4', m4v: 'video/mp4', mov: 'video/quicktime',
            webm: 'video/webm', ogv: 'video/ogg',
            mp3: 'audio/mpeg', m4a: 'audio/mp4', ogg: 'audio/ogg',
            wav: 'audio/wav', flac: 'audio/flac', aac: 'audio/aac',
        };
        return map[ext] || 'application/octet-stream';
    }

    /**
     * Save a downloaded file to disk.
     * Waits for all required pieces, assembles a Blob, then triggers a browser
     * download using the File System Access API (showSaveFilePicker, Chrome 86+)
     * or falls back to an <a download> click.
     * @param {number} fileIndex
     */
    async saveFile(fileIndex = 0) {
        const file = this.files[fileIndex];
        if (!file) throw new Error(`File at index ${fileIndex} not found`);
        const startPiece = Math.floor(file.offset / this.pieceLength);
        const endPiece = Math.ceil((file.offset + file.length) / this.pieceLength);
        for (let i = startPiece; i < endPiece; i++) {
            await this._waitForPiece(i);
        }
        const blob = await this.getBlob(file);
        const filename = (file.name || 'download').split('/').pop();
        await this._saveBlob(blob, filename);
    }

    async _saveBlob(blob, filename) {
        if (typeof window === 'undefined') return;
        if (window.showSaveFilePicker) {
            try {
                const fh = await window.showSaveFilePicker({ suggestedName: filename });
                const writable = await fh.createWritable();
                await writable.write(blob);
                await writable.close();
                return;
            } catch (e) {
                if (e.name === 'AbortError') return; // user cancelled
                console.warn('[RtcTorrent] showSaveFilePicker failed, using <a> fallback:', e.message);
            }
        }
        const url = URL.createObjectURL(blob);
        const a = Object.assign(document.createElement('a'), { href: url, download: filename });
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        setTimeout(() => URL.revokeObjectURL(url), 30000);
    }

    /**
     * Check whether a filename has a browser-playable media type.
     * Uses canPlayType() on a temporary element for accurate detection.
     * @param {string} filename
     * @returns {boolean}
     */
    isPlayable(filename) {
        if (typeof window === 'undefined') return false;
        const ext = ((filename || '').split('/').pop()?.split('.').pop() || '').toLowerCase();
        const map = {
            mp4: 'video/mp4', m4v: 'video/mp4', mov: 'video/quicktime',
            webm: 'video/webm', ogv: 'video/ogg',
            mp3: 'audio/mpeg', m4a: 'audio/mp4', ogg: 'audio/ogg',
            wav: 'audio/wav', flac: 'audio/flac', aac: 'audio/aac',
            opus: 'audio/ogg; codecs=opus',
        };
        const mime = map[ext];
        if (!mime) return false;
        const tag = mime.startsWith('audio/') ? 'audio' : 'video';
        return document.createElement(tag).canPlayType(mime) !== '';
    }
}

class SignalChannel extends EventTarget {
    constructor(trackerUrl, infoHash) {
        super();
        this.trackerUrl = trackerUrl;
        this.infoHash = infoHash;
    }

    _urlEncodeBytes(bytes) {
        let out = '';
        for (let i = 0; i < bytes.length; i++) out += '%' + bytes[i].toString(16).padStart(2, '0');
        return out;
    }

    _hexToBytes(hex) {
        const b = new Uint8Array(hex.length / 2);
        for (let i = 0; i < b.length; i++) b[i] = parseInt(hex.substr(i * 2, 2), 16);
        return b;
    }

    _peerId() {
        let id = '-RT1000-';
        for (let i = 0; i < 12; i++) id += Math.floor(Math.random() * 10);
        return id;
    }

    async sendOffer(sdpOffer, _peerId) {
        const infoHashBytes = this._hexToBytes(this.infoHash);
        const parts = [
            'info_hash=' + this._urlEncodeBytes(infoHashBytes),
            'peer_id=' + encodeURIComponent(this._peerId()),
            'rtcoffer=' + encodeURIComponent(sdpOffer),
            'rtctorrent=1',
            'event=started',
            'port=6881', 'uploaded=0', 'downloaded=0', 'left=0', 'compact=1'
        ];
        await _fetch(`${this.trackerUrl}?${parts.join('&')}`).catch(() => {});
    }

    async sendAnswer(sdpAnswer) {
        const infoHashBytes = this._hexToBytes(this.infoHash);
        const parts = [
            'info_hash=' + this._urlEncodeBytes(infoHashBytes),
            'peer_id=' + encodeURIComponent(this._peerId()),
            'rtcanswer=' + encodeURIComponent(sdpAnswer),
            'rtctorrent=1',
            'event=started',
            'port=6881', 'uploaded=0', 'downloaded=0', 'left=0', 'compact=1'
        ];
        await _fetch(`${this.trackerUrl}?${parts.join('&')}`).catch(() => {});
    }

    async sendICECandidate(_candidate, _peerId) {}

    close() {}

    emit(name, data) {
        this.dispatchEvent(new CustomEvent(name, { detail: data }));
    }

    on(name, cb) {
        this.addEventListener(name, e => cb(e.detail));
    }
}

class WebRTCManager {
    constructor(options) {
        this.options = options;
        this.connections = new Map();
        this.nextId = 0;
    }

    createConnection(config = {}) {
        const RTCPeerConnection = (() => {
            if (typeof globalThis.RTCPeerConnection !== 'undefined') return globalThis.RTCPeerConnection;
            const nativeRequire = (typeof __non_webpack_require__ !== 'undefined')
                ? __non_webpack_require__
                : require;
            for (const pkg of ['wrtc', '@roamhq/wrtc', 'node-webrtc']) {
                try { const m = nativeRequire(pkg); if (m.RTCPeerConnection) return m.RTCPeerConnection; } catch (_) {}
            }
            throw new Error('RTCPeerConnection not available. Install "wrtc" for Node.js support.');
        })();
        const pc = new RTCPeerConnection({
            iceServers: this.options.iceServers || [{ urls: 'stun:stun.l.google.com:19302' }],
            ...config
        });
        const id = `conn_${this.nextId++}`;
        this.connections.set(id, pc);
        pc.onconnectionstatechange = () => {
            if (pc.connectionState === 'closed' || pc.connectionState === 'failed') {
                this.connections.delete(id);
            }
        };
        return pc;
    }

    async close() {
        for (const [, pc] of this.connections) { try { pc.close(); } catch (_) {} }
        this.connections.clear();
    }
}

class Announcer {
    constructor(options) { this.options = options; }
    async announce(_torrent, _params) {}
}

if (typeof module !== 'undefined' && module.exports) {
    module.exports = RtcTorrent;
} else if (typeof window !== 'undefined') {
    window.RtcTorrent = RtcTorrent;
}

export default RtcTorrent;