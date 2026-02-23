const fs = require('fs');
const path = require('path');
const os = require('os');

const CONFIG = {
    trackerUrl: process.env.TRACKER_URL || 'http://127.0.0.1:6969/announce',
    announceInterval: parseInt(process.env.ANNOUNCE_INTERVAL) || 30000,
    rtcInterval: parseInt(process.env.RTC_INTERVAL) || 10000
};

console.log(`Using tracker URL: ${CONFIG.trackerUrl}`);

async function leechTorrent(torrentPathOrMagnet, outputDir = null) {
    try {
        console.log('Starting to download torrent...');
        let RtcTorrent;
        try {
            const builtLib = require('../dist/rtctorrent.node.js');
            RtcTorrent = builtLib.default || builtLib.RtcTorrent || builtLib;
        } catch (e) {
            console.error('Error loading RtcTorrent library:', e.message);
            throw new Error('Cannot load RtcTorrent library for Node.js. Make sure to build it first with `npm run build`');
        }
        const downloadDir = outputDir || path.join(os.tmpdir(), 'rtctorrent-downloads');
        if (!fs.existsSync(downloadDir)) {
            fs.mkdirSync(downloadDir, { recursive: true });
            console.log(`Created download directory: ${downloadDir}`);
        }
        const client = new RtcTorrent({
            trackerUrl: CONFIG.trackerUrl,
            announceInterval: CONFIG.announceInterval,
            rtcInterval: CONFIG.rtcInterval
        });
        await client.start();
        let torrentData;
        if (torrentPathOrMagnet.startsWith('magnet:')) {
            torrentData = torrentPathOrMagnet;
            console.log('Downloading from magnet URI...');
        } else {
            if (!fs.existsSync(torrentPathOrMagnet)) {
                throw new Error(`Torrent file does not exist: ${torrentPathOrMagnet}`);
            }
            const torrentBuffer = fs.readFileSync(torrentPathOrMagnet);
            torrentData = torrentBuffer;
            console.log(`Downloading from torrent file: ${torrentPathOrMagnet}`);
        }
        console.log('Starting download...');
        const torrent = await client.download(torrentData);
        console.log('Download started!');
        console.log('Info Hash:', torrent.data.infoHash);
        const intervalId = setInterval(() => {
            const progress = (torrent.downloaded / torrent.totalSize) * 100;
            console.log(`Progress: ${progress.toFixed(2)}% (${torrent.downloaded}/${torrent.totalSize} bytes)`);
            if (torrent.downloaded >= torrent.totalSize) {
                console.log('Download completed!');
                clearInterval(intervalId);
                console.log(`Files would be saved to: ${downloadDir}`);
                if (torrent.files && torrent.files.length > 0) {
                    console.log('Files in torrent:');
                    torrent.files.forEach((file, index) => {
                        console.log(`  ${index + 1}. ${file.name} (${file.length} bytes)`);
                    });
                }
            }
        }, 5000);
        return new Promise((resolve) => {
            setTimeout(() => {
                console.log('Download simulation completed');
                resolve(torrent);
            }, 30000);
        });
    } catch (error) {
        console.error('Error downloading torrent:', error);
        throw error;
    }
}

if (require.main === module) {
    const args = process.argv.slice(2);
    if (args.length < 1) {
        console.log('Usage: node leech_torrent.js <torrent_file_or_magnet_uri> [output_directory]');
        console.log('Example: node leech_torrent.js demo_video.torrent');
        console.log('Example: node leech_torrent.js "magnet:?xt=urn:btih:..." ./downloads');
        console.log('');
        console.log('Environment variables:');
        console.log('  TRACKER_URL         - Tracker URL (default: http://127.0.0.1:6969/announce)');
        console.log('  ANNOUNCE_INTERVAL - Announce interval in ms (default: 30000)');
        console.log('  RTC_INTERVAL      - RTC interval in ms (default: 10000)');
        console.log('');
        console.log('Examples with custom tracker:');
        console.log('  TRACKER_URL=http://my-tracker.com:6969/announce node leech_torrent.js demo_video.torrent');
        console.log('  TRACKER_URL=https://secure-tracker.org:443/announce ANNOUNCE_INTERVAL=15000 node leech_torrent.js demo_video.torrent ./downloads');
        process.exit(1);
    }
    const torrentPathOrMagnet = args[0];
    const outputDirectory = args[1] || null;
    leechTorrent(torrentPathOrMagnet, outputDirectory).catch(console.error);
}

module.exports = leechTorrent;