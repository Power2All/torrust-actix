const fs = require('fs');
const path = require('path');

const CONFIG = {
    trackerUrl: process.env.TRACKER_URL || 'http://127.0.0.1:6969/announce',
    announceInterval: parseInt(process.env.ANNOUNCE_INTERVAL) || 30000,
    rtcInterval: parseInt(process.env.RTC_INTERVAL) || 10000
};

console.log(`Using tracker URL: ${CONFIG.trackerUrl}`);

async function seedTorrent(torrentPathOrMagnet) {
    try {
        console.log('Starting to seed torrent...');
        let RtcTorrent;
        try {
            const builtLib = require('../dist/rtctorrent.node.js');
            RtcTorrent = builtLib.default || builtLib.RtcTorrent || builtLib;
        } catch (e) {
            console.error('Error loading RtcTorrent library:', e.message);
            throw new Error('Cannot load RtcTorrent library for Node.js. Make sure to build it first with `npm run build`');
        }
        const client = new RtcTorrent({
            trackerUrl: CONFIG.trackerUrl,
            announceInterval: CONFIG.announceInterval,
            rtcInterval: CONFIG.rtcInterval
        });
        await client.start();
        let torrentData;
        let fileToSeed = null;
        if (torrentPathOrMagnet.startsWith('magnet:')) {
            torrentData = torrentPathOrMagnet;
            console.log('Seeding from magnet URI...');
        } else {
            if (!fs.existsSync(torrentPathOrMagnet)) {
                throw new Error(`Torrent file does not exist: ${torrentPathOrMagnet}`);
            }
            const torrentBuffer = fs.readFileSync(torrentPathOrMagnet);
            torrentData = torrentBuffer;
            console.log(`Seeding from torrent file: ${torrentPathOrMagnet}`);
            const actualFilePath = path.join(path.dirname(torrentPathOrMagnet), 'file_example_MP4_1920_18MG.mp4');
            if (fs.existsSync(actualFilePath)) {
                fileToSeed = [actualFilePath];
                console.log(`Found actual file to seed: ${actualFilePath}`);
            } else {
                console.log('Note: Actual file not found for seeding - this may cause seeding to fail');
            }
        }
        const torrent = await client.seed(torrentData, fileToSeed);
        console.log('Successfully started seeding!');
        console.log('Info Hash:', torrent.data.infoHash);
        setInterval(() => {
            console.log(`Currently seeding, peers: ${torrent.peers.size || 'unknown'}`);
        }, 10000);
        return torrent;
    } catch (error) {
        console.error('Error seeding torrent:', error);
        throw error;
    }
}

if (require.main === module) {
    const args = process.argv.slice(2);
    if (args.length < 1) {
        console.log('Usage: node seed_torrent.js <torrent_file_or_magnet_uri>');
        console.log('Example: node seed_torrent.js demo_video.torrent');
        console.log('Example: node seed_torrent.js "magnet:?xt=urn:btih:..."');
        console.log('');
        console.log('Environment variables:');
        console.log('  TRACKER_URL         - Tracker URL (default: http://127.0.0.1:6969/announce)');
        console.log('  ANNOUNCE_INTERVAL - Announce interval in ms (default: 30000)');
        console.log('  RTC_INTERVAL      - RTC interval in ms (default: 10000)');
        console.log('');
        console.log('Examples with custom tracker:');
        console.log('  TRACKER_URL=http://my-tracker.com:6969/announce node seed_torrent.js demo_video.torrent');
        console.log('  TRACKER_URL=https://secure-tracker.org:443/announce ANNOUNCE_INTERVAL=15000 node seed_torrent.js demo_video.torrent');
        process.exit(1);
    }
    const torrentPathOrMagnet = args[0];
    seedTorrent(torrentPathOrMagnet).catch(console.error);
}

module.exports = seedTorrent;