# RtcTorrent Test Scripts

These test scripts demonstrate the functionality of the RtcTorrent library for creating, seeding, and downloading torrents using WebRTC-enhanced BitTorrent protocol.

## Prerequisites

- Node.js installed
- A running BitTorrent tracker (default: `http://127.0.0.1:6969/announce`)
- A file of your choice to use with `create_torrent.js`

## Configuration

All test scripts support the following environment variables:

- `TRACKER_URL` - Custom tracker URL (default: `http://127.0.0.1:6969/announce`)
- `ANNOUNCE_INTERVAL` - Announce interval in milliseconds (default: 30000)
- `RTC_INTERVAL` - RTC (WebRTC) interval in milliseconds (default: 10000)

Examples:
```bash
# Use custom tracker
TRACKER_URL=http://my-tracker.com:6969/announce node seed_torrent.js demo_video.torrent

# Custom intervals
TRACKER_URL=https://secure-tracker.org:443/announce ANNOUNCE_INTERVAL=15000 RTC_INTERVAL=8000 node leech_torrent.js demo_video.torrent
```

## Scripts

### 1. create_torrent.js
Creates a torrent file from any file you provide.

**Usage:**
```bash
node create_torrent.js <file>
```

Examples:
```bash
node create_torrent.js /path/to/video.mp4
node create_torrent.js ~/Downloads/movie.mkv
```

This script will:
- Create a `.torrent` file in the same directory as the input file
- Print the info hash and magnet URI

### 2. seed_torrent.js
Starts seeding a torrent (either from a .torrent file or magnet URI).

**Usage:**
```bash
node seed_torrent.js <torrent_file_or_magnet_uri>
```

Examples:
```bash
node seed_torrent.js demo_video.torrent
node seed_torrent.js "magnet:?xt=urn:btih:..."
```

This script will:
- Start seeding the specified torrent
- Announce to the tracker regularly
- Log connection status

### 3. leech_torrent.js
Downloads a torrent (either from a .torrent file or magnet URI).

**Usage:**
```bash
node leech_torrent.js <torrent_file_or_magnet_uri> [output_directory]
```

Examples:
```bash
node leech_torrent.js demo_video.torrent
node leech_torrent.js "magnet:?xt=urn:btih:..." ./downloads
```

This script will:
- Start downloading the specified torrent
- Monitor download progress
- Save files to the specified output directory (defaults to system temp directory)

## Testing Workflow

1. Ensure a tracker is running at `http://127.0.0.1:6969/announce` (or set `TRACKER_URL`)
2. Run `create_torrent.js <your_file>` to create a torrent file
3. On one terminal, run `seed_torrent.js <your_file>.torrent`
4. On another terminal, run `leech_torrent.js <your_file>.torrent`

## Note

These are test scripts that demonstrate the RtcTorrent functionality. In a real-world scenario, you would have more sophisticated error handling, file management, and progress reporting.