const fs = require('fs');
const path = require('path');

let RtcTorrent;
try {
    const builtLib = require('../dist/rtctorrent.node.js');
    RtcTorrent = builtLib.default || builtLib.RtcTorrent || builtLib;
} catch (e) {
    console.error('Error loading Node.js RtcTorrent library:', e.message);
    try {
        const builtLib = require('../dist/rtctorrent.browser.js');
        RtcTorrent = builtLib.default || builtLib.RtcTorrent || builtLib;
    } catch (e2) {
        console.error('Error loading browser RtcTorrent library:', e2.message);
        try {
            const srcPath = path.join(__dirname, '..', 'src', 'rtctorrent.js');
            const srcCode = fs.readFileSync(srcPath, 'utf8');
            const transformedCode = srcCode
                .replace(/export default RtcTorrent;/g, '')
                .replace(/export.+RtcTorrent.*/g, '') +
                'module.exports = { default: RtcTorrent, RtcTorrent };';
            const moduleExports = {};
            const context = {
                module: { exports: moduleExports },
                exports: moduleExports,
                require: require,
                __filename: srcPath,
                __dirname: path.dirname(srcPath),
                console: console,
                setTimeout: setTimeout,
                clearTimeout: clearTimeout,
                setImmediate: setImmediate,
                clearImmediate: clearImmediate,
                setInterval: setInterval,
                clearInterval: clearInterval,
                window: undefined,
                self: undefined
            };
            const vm = require('vm');
            vm.runInNewContext(transformedCode, context, { filename: srcPath });
            RtcTorrent = context.module.exports.default || context.module.exports.RtcTorrent;
        } catch (e3) {
            console.error('Fallback also failed:', e3.message);
            throw new Error('Cannot load RtcTorrent library for Node.js');
        }
    }
}

async function createTorrent(inputFile) {
    try {
        const filePath = inputFile
            ? path.resolve(inputFile)
            : process.argv[2] && path.resolve(process.argv[2]);
        if (!filePath) {
            console.error('Usage: node create_torrent.js <file>');
            console.error('Example: node create_torrent.js /path/to/video.mp4');
            process.exit(1);
        }
        if (!fs.existsSync(filePath)) {
            console.error('File not found:', filePath);
            process.exit(1);
        }
        console.log('Creating torrent from:', filePath);
        const client = new RtcTorrent({
            trackerUrl: process.env.TRACKER_URL || 'http://127.0.0.1:6969/announce',
            announceInterval: 30000,
            rtcInterval: 10000
        });
        const fileObject = {
            path: filePath,
            name: path.basename(filePath),
            size: fs.statSync(filePath).size
        };
        const torrentData = await client.create([fileObject], {
            name: path.basename(filePath, path.extname(filePath)),
            comment: 'Created with RtcTorrent'
        });
        console.log('Torrent created successfully!');
        console.log('Info Hash:', torrentData.infoHash);
        console.log('Magnet URI:', torrentData.magnetUri);
        const torrentFileName = path.join(
            path.dirname(filePath),
            path.basename(filePath, path.extname(filePath)) + '.torrent'
        );
        if (torrentData.encodedTorrent && Buffer.isBuffer(torrentData.encodedTorrent)) {
            fs.writeFileSync(torrentFileName, torrentData.encodedTorrent);
        } else if (torrentData.encodedTorrent && typeof torrentData.encodedTorrent === 'string') {
            fs.writeFileSync(torrentFileName, torrentData.encodedTorrent);
        } else if (torrentData.torrent) {
            const bencoder = require('bencode');
            const encoded = bencoder.encode(torrentData.torrent);
            fs.writeFileSync(torrentFileName, encoded);
        } else {
            console.log('Warning: Unable to properly encode torrent file');
            fs.writeFileSync(torrentFileName, JSON.stringify(torrentData.torrent || {}, null, 2));
        }
        console.log(`Torrent file saved as: ${torrentFileName}`);
        return torrentData;
    } catch (error) {
        console.error('Error creating torrent:', error);
        console.error('Stack:', error.stack);
        throw error;
    }
}

if (require.main === module) {
    createTorrent().catch(console.error);
}

module.exports = createTorrent;