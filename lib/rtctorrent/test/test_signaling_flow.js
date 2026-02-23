/**
 * End-to-end test for the HTTP-based WebRTC signaling protocol.
 * Tests all three steps WITHOUT requiring RTCPeerConnection:
 *   Step 1: Seeder registers with SDP offer → tracker stores it
 *   Step 2: Leecher gets seeder's offer from tracker → sends back SDP answer
 *   Step 3: Seeder re-announces → receives the pending answer
 *
 * Run: node test_signaling_flow.js
 * Requires tracker running at http://127.0.0.1:6969
 */

const http = require('http');

const TRACKER = 'http://127.0.0.1:6969/announce';

const INFO_HASH_HEX = 'aabbccddeeff00112233445566778899aabbccdd';
const SEEDER_PEER_ID_HEX = '2d5254313030302d534545444552303030303031';
const LEECHER_PEER_ID_HEX = '2d525443313030302d4c45454348303030303031';

function hexToUrlEncoded(hex) {
    let result = '';
    for (let i = 0; i < hex.length; i += 2) {
        result += '%' + hex.slice(i, i + 2);
    }
    return result;
}

function get(url) {
    return new Promise((resolve, reject) => {
        http.get(url, (res) => {
            const chunks = [];
            res.on('data', c => chunks.push(c));
            res.on('end', () => resolve({ status: res.statusCode, body: Buffer.concat(chunks) }));
        }).on('error', reject);
    });
}

function bencodeDecode(buf, pos = 0) {
    const b = buf[pos];
    if (b === 0x69) { // 'i'
        const end = buf.indexOf(0x65, pos + 1); // 'e'
        const val = parseInt(buf.slice(pos + 1, end).toString('ascii'));
        return [val, end + 1];
    }
    if (b === 0x6c) { // 'l'
        pos++;
        const list = [];
        while (buf[pos] !== 0x65) { // 'e'
            const [item, newPos] = bencodeDecode(buf, pos);
            list.push(item);
            pos = newPos;
        }
        return [list, pos + 1];
    }
    if (b === 0x64) { // 'd'
        pos++;
        const dict = {};
        while (buf[pos] !== 0x65) { // 'e'
            const [key, p1] = bencodeDecode(buf, pos);
            const [val, p2] = bencodeDecode(buf, p1);
            dict[typeof key === 'string' ? key : key.toString('ascii')] = val;
            pos = p2;
        }
        return [dict, pos + 1];
    }
    const colon = buf.indexOf(0x3a, pos); // ':'
    const len = parseInt(buf.slice(pos, colon).toString('ascii'));
    const data = buf.slice(colon + 1, colon + 1 + len);
    return [data, colon + 1 + len];
}

function decode(buf) {
    return bencodeDecode(buf, 0)[0];
}

function buildAnnounceUrl(params) {
    const parts = [];
    for (const [k, v] of Object.entries(params)) {
        if (v !== undefined && v !== null) {
            parts.push(k + '=' + v);
        }
    }
    return TRACKER + '?' + parts.join('&');
}

async function run() {
    let passed = 0;
    let failed = 0;
    function ok(name, cond, detail = '') {
        if (cond) {
            console.log(`  ✓ ${name}`);
            passed++;
        } else {
            console.error(`  ✗ ${name}${detail ? ': ' + detail : ''}`);
            failed++;
        }
    }
    const FAKE_SDP_OFFER = 'v=0\r\no=- 1 1 IN IP4 127.0.0.1\r\ns=offer\r\nt=0 0\r\n';
    const FAKE_SDP_ANSWER = 'v=0\r\no=- 2 2 IN IP4 127.0.0.1\r\ns=answer\r\nt=0 0\r\n';
    const infoHashEnc = hexToUrlEncoded(INFO_HASH_HEX);
    const seederEnc = hexToUrlEncoded(SEEDER_PEER_ID_HEX);
    const leecherEnc = hexToUrlEncoded(LEECHER_PEER_ID_HEX);
    console.log('\n--- Step 1: Seeder registers SDP offer ---');
    const step1Url = buildAnnounceUrl({
        info_hash: infoHashEnc,
        peer_id: seederEnc,
        port: '6881',
        uploaded: '0',
        downloaded: '0',
        left: '0',
        compact: '1',
        event: 'started',
        numwant: '50',
        rtctorrent: '1',
        rtcoffer: encodeURIComponent(FAKE_SDP_OFFER)
    });
    console.log('  URL:', step1Url.slice(0, 120) + '...');
    const r1 = await get(step1Url);
    console.log('  HTTP status:', r1.status);
    console.log('  Raw response:', r1.body.toString('utf8').slice(0, 200));
    ok('Step 1: HTTP 200', r1.status === 200);
    let d1;
    try {
        d1 = decode(r1.body);
        ok('Step 1: response is a bencode dict', typeof d1 === 'object' && !Buffer.isBuffer(d1));
    } catch (e) {
        ok('Step 1: bencode parse', false, e.message);
    }
    console.log('\n--- Step 2: Leecher gets offer + sends answer ---');
    const step2aUrl = buildAnnounceUrl({
        info_hash: infoHashEnc,
        peer_id: leecherEnc,
        port: '6882',
        uploaded: '0',
        downloaded: '0',
        left: '1000000',
        compact: '1',
        event: 'started',
        numwant: '50',
        rtctorrent: '1',
        rtcrequest: '1'
    });
    console.log('  URL:', step2aUrl.slice(0, 120) + '...');
    const r2a = await get(step2aUrl);
    console.log('  HTTP status:', r2a.status);
    console.log('  Raw response:', r2a.body.toString('utf8').slice(0, 400));
    ok('Step 2a: HTTP 200', r2a.status === 200);
    let d2a;
    try {
        d2a = decode(r2a.body);
        ok('Step 2a: bencode parse ok', true);
    } catch (e) {
        ok('Step 2a: bencode parse', false, e.message);
        d2a = {};
    }
    const rtcPeers = d2a['rtc_peers'] || d2a['rtc_peers\n'];
    const rtcPeersArr = Array.isArray(rtcPeers) ? rtcPeers : [];
    console.log('  rtc_peers count:', rtcPeersArr.length);
    if (rtcPeersArr.length > 0) {
        const first = rtcPeersArr[0];
        const peerIdBuf = Buffer.isBuffer(first['peer_id']) ? first['peer_id'] : Buffer.from(first['peer_id'] || '');
        const sdpOfferBuf = Buffer.isBuffer(first['sdp_offer']) ? first['sdp_offer'] : Buffer.from(first['sdp_offer'] || '');
        console.log('  First peer_id (hex):', peerIdBuf.toString('hex'));
        console.log('  First sdp_offer:', sdpOfferBuf.toString('utf8').slice(0, 80));
        ok('Step 2a: rtc_peers has 1+ entries', rtcPeersArr.length >= 1);
        ok('Step 2a: seeder peer_id matches', peerIdBuf.toString('hex') === SEEDER_PEER_ID_HEX);
        ok('Step 2a: sdp_offer received', sdpOfferBuf.toString('utf8').includes('v=0'));
    } else {
        ok('Step 2a: rtc_peers has 1+ entries', false, 'empty array');
        ok('Step 2a: seeder peer_id matches', false, 'no peers');
        ok('Step 2a: sdp_offer received', false, 'no peers');
    }
    console.log('\n  Leecher sending SDP answer for seeder...');
    const step2bUrl = buildAnnounceUrl({
        info_hash: infoHashEnc,
        peer_id: leecherEnc,
        port: '6882',
        uploaded: '0',
        downloaded: '0',
        left: '1000000',
        compact: '1',
        rtctorrent: '1',
        rtcanswer: encodeURIComponent(FAKE_SDP_ANSWER),
        rtcanswerfor: SEEDER_PEER_ID_HEX
    });
    const r2b = await get(step2bUrl);
    console.log('  HTTP status:', r2b.status);
    ok('Step 2b: HTTP 200', r2b.status === 200);
    console.log('\n--- Step 3: Seeder polls for answers ---');
    const step3Url = buildAnnounceUrl({
        info_hash: infoHashEnc,
        peer_id: seederEnc,
        port: '6881',
        uploaded: '0',
        downloaded: '0',
        left: '0',
        compact: '1',
        rtctorrent: '1',
        rtcoffer: encodeURIComponent(FAKE_SDP_OFFER)
    });
    const r3 = await get(step3Url);
    console.log('  HTTP status:', r3.status);
    console.log('  Raw response:', r3.body.toString('utf8').slice(0, 400));
    ok('Step 3: HTTP 200', r3.status === 200);
    let d3;
    try {
        d3 = decode(r3.body);
        ok('Step 3: bencode parse ok', true);
    } catch (e) {
        ok('Step 3: bencode parse', false, e.message);
        d3 = {};
    }
    const rtcAnswers = d3['rtc_answers'] || d3['rtc_answers\n'];
    const rtcAnswersArr = Array.isArray(rtcAnswers) ? rtcAnswers : [];
    console.log('  rtc_answers count:', rtcAnswersArr.length);
    if (rtcAnswersArr.length > 0) {
        const first = rtcAnswersArr[0];
        const peerIdBuf = Buffer.isBuffer(first['peer_id']) ? first['peer_id'] : Buffer.from(first['peer_id'] || '');
        const sdpAnswerBuf = Buffer.isBuffer(first['sdp_answer']) ? first['sdp_answer'] : Buffer.from(first['sdp_answer'] || '');
        console.log('  First peer_id (hex):', peerIdBuf.toString('hex'));
        console.log('  First sdp_answer:', sdpAnswerBuf.toString('utf8').slice(0, 80));
        ok('Step 3: rtc_answers has 1+ entries', rtcAnswersArr.length >= 1);
        ok('Step 3: answerer peer_id matches leecher', peerIdBuf.toString('hex') === LEECHER_PEER_ID_HEX);
        ok('Step 3: sdp_answer received', sdpAnswerBuf.toString('utf8').includes('v=0'));
    } else {
        ok('Step 3: rtc_answers has 1+ entries', false, 'empty array — pending answers were lost!');
        ok('Step 3: answerer peer_id matches leecher', false, 'no answers');
        ok('Step 3: sdp_answer received', false, 'no answers');
    }
    console.log(`\n=== Results: ${passed} passed, ${failed} failed ===\n`);
    process.exit(failed > 0 ? 1 : 0);
}

run().catch(err => {
    console.error('Unhandled error:', err);
    process.exit(2);
});