/*!
 * RtcTorrent Embeddable Player  –  embed.js
 *
 * Drop-in video player powered by WebRTC torrenting.
 * Requires rtctorrent.browser.js to be loaded first.
 *
 * ── Quick start ─────────────────────────────────────────────────────────────
 *
 *   <!-- 1. A container with an explicit size -->
 *   <div id="player" style="width:100%;max-width:960px;aspect-ratio:16/9"></div>
 *
 *   <!-- 2. Library + embed script -->
 *   <script src="dist/rtctorrent.browser.js"></script>
 *   <script src="demo/embed.js"></script>
 *
 *   <!-- 3. Create the player -->
 *   <script>
 *     RtcTorrentPlayer('#player', {
 *       torrent: 'https://cdn.example.com/file.torrent',
 *       tracker: 'https://tracker.example.com/announce',
 *       swPath:  '/sw.js',   // copy sw.js to your server root for seamless streaming
 *     });
 *   </script>
 *
 * ── All options ──────────────────────────────────────────────────────────────
 *
 *   torrent       {string}    URL to a .torrent file            (required if no magnet)
 *   magnet        {string}    magnet: URI                       (required if no torrent)
 *   tracker       {string}    Announce URL                      default: 'http://127.0.0.1:6969/announce'
 *   file          {number}    File index inside the torrent     default: 0
 *   swPath        {string}    Path to sw.js on your server      default: null (blob fallback)
 *   autoplay      {boolean}   Try to autoplay on load           default: true
 *   muted         {boolean}   Start muted (helps autoplay)      default: false
 *   initialExtra  {number}    Extra pieces to buffer before     default: 50  (~3 MB at 64 KB/piece)
 *                             playback starts
 *   rtcInterval   {number}    RTC announce interval (ms)        default: 5000
 *   iceServers    {array}     WebRTC ICE server list            default: Google STUN
 *   statsInterval {number}    Console stats interval (ms)       default: 5000
 *   onStats       {function}  Called each stats tick            default: null
 *                             ({ downloaded, total, percent, pieces, totalPieces, peers, speed })
 *   onError       {function}  Called on fatal error (msg)       default: null
 *   onReady       {function}  Called when torrent metadata is   default: null
 *                             ready, receives the player instance
 *
 * ── Instance methods ────────────────────────────────────────────────────────
 *
 *   player.destroy()   Stop streaming and remove the player from the DOM.
 */

(function (global) {
  'use strict';

  const VJS_VERSION = '8.23.4';
  const VJS_CSS = `https://vjs.zencdn.net/${VJS_VERSION}/video-js.css`;
  const VJS_JS  = `https://vjs.zencdn.net/${VJS_VERSION}/video.min.js`;

  /* ── Tiny helpers ─────────────────────────────────────────────────────── */

  function loadCSS(href) {
    if (document.querySelector(`link[href="${href}"]`)) return;
    document.head.appendChild(
      Object.assign(document.createElement('link'), { rel: 'stylesheet', href })
    );
  }

  function loadScript(src) {
    if (window.videojs) return Promise.resolve();
    if (document.querySelector(`script[src="${src}"]`)) return Promise.resolve();
    return new Promise((res, rej) => {
      const el = Object.assign(document.createElement('script'), { src });
      el.onload = res;
      el.onerror = () => rej(new Error('Failed to load: ' + src));
      document.head.appendChild(el);
    });
  }

  function fmt(bytes) {
    if (!bytes || bytes <= 0) return '0 B';
    if (bytes < 1024)        return bytes + ' B';
    if (bytes < 1048576)     return (bytes / 1024).toFixed(1) + ' KB';
    if (bytes < 1073741824)  return (bytes / 1048576).toFixed(1) + ' MB';
    return (bytes / 1073741824).toFixed(2) + ' GB';
  }

  /* ── Shared stylesheet (injected once per page) ───────────────────────── */

  function injectStyles() {
    if (document.getElementById('__rtcp_css__')) return;
    const el = document.createElement('style');
    el.id = '__rtcp_css__';
    el.textContent = `
      /* ── root container ── */
      .rtcp-root {
        position: relative; width: 100%; height: 100%;
        background: #000; overflow: hidden;
      }
      .rtcp-root .video-js { width: 100% !important; height: 100% !important; }

      /* ── thin download-progress strip at the very bottom ── */
      .rtcp-dl-bar  { position:absolute;bottom:0;left:0;right:0;height:3px;z-index:15;background:rgba(255,255,255,.07);pointer-events:none; }
      .rtcp-dl-fill { height:100%;width:0%;background:#3b82f6;transition:width .5s linear; }

      /* ── loading / buffering overlay ── */
      .rtcp-overlay {
        position: absolute; inset: 0; z-index: 20;
        display: flex; flex-direction: column;
        align-items: center; justify-content: center;
        gap: 10px; text-align: center; padding: 24px;
        background: rgba(0,0,0,.8);
        font-family: system-ui, -apple-system, BlinkMacSystemFont, sans-serif;
        transition: opacity .3s ease;
      }
      .rtcp-overlay.fade-out { opacity: 0; pointer-events: none; }
      .rtcp-overlay.gone     { display: none; }

      /* spinner */
      .rtcp-spinner {
        width: 36px; height: 36px; flex-shrink: 0;
        border: 3px solid rgba(255,255,255,.12); border-top-color: #3b82f6;
        border-radius: 50%; animation: rtcp-spin .75s linear infinite;
      }
      @keyframes rtcp-spin { to { transform: rotate(360deg); } }

      /* text rows */
      .rtcp-title  { font-size: .9rem;  font-weight: 600; color: #fff;           max-width: 340px; word-break: break-all; line-height: 1.35; }
      .rtcp-status { font-size: .76rem; color: #93c5fd; letter-spacing: .01em;  }
      .rtcp-live   { font-size: .68rem; color: rgba(255,255,255,.38); font-variant-numeric: tabular-nums; margin-top: -2px; }

      /* error state */
      .rtcp-overlay.error .rtcp-title  { color: #fca5a5; }
      .rtcp-overlay.error .rtcp-status { color: #d1d5db; white-space: pre-wrap; font-size: .78rem; }
    `;
    document.head.appendChild(el);
  }

  /* ════════════════════════════════════════════════════════════════════════
     Player class
     ════════════════════════════════════════════════════════════════════════ */

  class RtcTorrentPlayerInstance {
    constructor(containerOrSelector, opts) {
      /* Resolve container */
      this._el = typeof containerOrSelector === 'string'
        ? document.querySelector(containerOrSelector)
        : containerOrSelector;
      if (!this._el)
        throw new Error(`[RtcTorrentPlayer] Container not found: "${containerOrSelector}"`);

      /* Merge defaults */
      this._o = Object.assign({
        torrent:       null,
        magnet:        null,
        tracker:       'http://127.0.0.1:6969/announce',
        file:          0,
        swPath:        null,
        autoplay:      true,
        muted:         false,
        initialExtra:  50,
        rtcInterval:   5000,
        iceServers: [
          { urls: 'stun:stun.l.google.com:19302'  },
          { urls: 'stun:stun1.l.google.com:19302' },
        ],
        statsInterval: 5000,
        onStats:  null,
        onError:  null,
        onReady:  null,
      }, opts);

      /* Internal state */
      this._torrent        = null;
      this._client         = null;
      this._vjs            = null;
      this._dlInterval     = null;
      this._statsInterval  = null;
      this._lastBytes      = 0;
      this._lastStatsTime  = 0;
      this._speed          = 0;
      this._videoId        = 'rtcp-' + Math.random().toString(36).slice(2, 9);

      this._boot();
    }

    /* ── Build DOM ──────────────────────────────────────────────────────── */
    _buildDOM() {
      injectStyles();
      this._el.innerHTML = `
        <div class="rtcp-root">
          <video id="${this._videoId}" class="video-js vjs-default-skin"
                 preload="none" playsinline${this._o.muted ? ' muted' : ''}></video>

          <div class="rtcp-dl-bar">
            <div class="rtcp-dl-fill"></div>
          </div>

          <div class="rtcp-overlay">
            <div class="rtcp-spinner"></div>
            <div class="rtcp-title">Loading torrent…</div>
            <div class="rtcp-status">Connecting to tracker</div>
            <div class="rtcp-live"></div>
          </div>
        </div>`;

      this._overlay  = this._el.querySelector('.rtcp-overlay');
      this._ovTitle  = this._el.querySelector('.rtcp-title');
      this._ovStatus = this._el.querySelector('.rtcp-status');
      this._ovLive   = this._el.querySelector('.rtcp-live');
      this._dlFill   = this._el.querySelector('.rtcp-dl-fill');
    }

    /* ── Overlay helpers ────────────────────────────────────────────────── */
    _status(msg)    { this._ovStatus.textContent = msg; }
    _liveStats(msg) { this._ovLive.textContent   = msg; }

    _hideOverlay() {
      this._overlay.classList.add('fade-out');
      setTimeout(() => this._overlay.classList.add('gone'), 380);
    }

    _showError(msg) {
      const sp = this._overlay.querySelector('.rtcp-spinner');
      if (sp) sp.remove();
      this._overlay.classList.remove('fade-out', 'gone');
      this._overlay.classList.add('error');
      this._ovTitle.textContent  = 'Error';
      this._ovStatus.textContent = msg;
      this._ovLive.textContent   = '';
      console.error('[RtcTorrentPlayer] Error:', msg);
      if (typeof this._o.onError === 'function') this._o.onError(msg);
    }

    /* ── Boot: load Video.js → register SW → run ─────────────────────────── */
    async _boot() {
      try {
        loadCSS(VJS_CSS);
        await loadScript(VJS_JS);
      } catch (_) {
        if (!window.videojs) { this._el.textContent = '[RtcTorrentPlayer] Video.js CDN unreachable'; return; }
      }

      if (this._o.swPath && 'serviceWorker' in navigator) {
        navigator.serviceWorker.register(this._o.swPath).catch(e =>
          console.warn('[RtcTorrentPlayer] SW unavailable — blob fallback will be used:', e.message)
        );
      }

      this._buildDOM();
      await this._run();
    }

    /* ── Main ────────────────────────────────────────────────────────────── */
    async _run() {
      const RtcTorrent = window.RtcTorrent;
      if (!RtcTorrent) { this._showError('RtcTorrent library not loaded.\nInclude rtctorrent.browser.js before embed.js.'); return; }

      const o = this._o;
      if (!o.torrent && !o.magnet) {
        this._showError('No source provided.\nSet { torrent: "url" } or { magnet: "magnet:..." }');
        return;
      }

      /* ── Init video.js player ────────────────────────────────────────── */
      this._vjs = videojs(this._videoId, {
        fill:           true,
        controls:       true,
        preload:        'none',
        bigPlayButton:  false,   // overlay handles pre-play UI
        loadingSpinner: false,   // overlay handles buffering UI
        errorDisplay:   false,   // we handle errors ourselves
      });
      await new Promise(r => this._vjs.ready(r));

      /* ── RtcTorrent client ───────────────────────────────────────────── */
      this._client = new RtcTorrent({
        trackerUrl:  o.tracker,
        rtcInterval: o.rtcInterval,
        iceServers:  o.iceServers,
      });

      /* ── Fetch / resolve torrent source ─────────────────────────────── */
      let source;
      if (o.torrent) {
        this._status('Fetching .torrent file…');
        try {
          const resp = await fetch(o.torrent);
          if (!resp.ok) throw new Error(`HTTP ${resp.status} from ${o.torrent}`);
          source = new Uint8Array(await resp.arrayBuffer());
        } catch (e) {
          this._showError('Failed to fetch torrent:\n' + e.message);
          return;
        }
      } else {
        source = o.magnet;
      }

      /* ── Parse torrent ───────────────────────────────────────────────── */
      this._status('Parsing torrent…');
      try {
        this._torrent = await this._client.download(source);
      } catch (e) {
        this._showError('Failed to load torrent:\n' + e.message);
        return;
      }

      const file = this._torrent.files[o.file];
      if (!file) {
        this._showError(
          `File index ${o.file} not found.\n` +
          `Torrent contains ${this._torrent.files.length} file(s).\n` +
          `Set { file: 0…${this._torrent.files.length - 1} }`
        );
        return;
      }

      const fname = file.name.split('/').pop();
      this._ovTitle.textContent = fname;
      this._status('Connecting to peers…');

      /* ── Console header ─────────────────────────────────────────────── */
      console.log(
        `[RtcTorrentPlayer] ▶ "${fname}"` +
        `  |  ${fmt(file.length)}` +
        `  |  ${this._torrent.pieceCount} pieces × ${fmt(this._torrent.pieceLength)}` +
        `  |  hash: ${this._torrent.data?.infoHash}`
      );

      /* ── Download progress bar ──────────────────────────────────────── */
      this._dlInterval = setInterval(() => {
        const t = this._torrent;
        if (t && t.totalSize > 0)
          this._dlFill.style.width = (t.downloaded / t.totalSize * 100).toFixed(1) + '%';
      }, 500);

      /* ── Periodic console stats ─────────────────────────────────────── */
      this._lastBytes     = 0;
      this._lastStatsTime = Date.now();
      this._statsInterval = setInterval(() => this._logStats(), o.statsInterval);

      this._torrent.onDownloadComplete = () => {
        clearInterval(this._dlInterval);
        clearInterval(this._statsInterval);
        this._dlFill.style.width = '100%';
        this._logStats(true);
      };

      /* ── onReady callback ───────────────────────────────────────────── */
      if (typeof o.onReady === 'function') o.onReady(this);

      /* ── Stream ─────────────────────────────────────────────────────── */
      const videoEl = this._vjs.el().querySelector('video');

      // Hide overlay as soon as the video has enough data to play
      this._vjs.one('canplay', () => this._hideOverlay());

      try {
        await this._torrent.streamVideo(o.file, videoEl, {
          initialExtra: o.initialExtra,
          onProgress: pct => {
            this._status(`Buffering… ${pct}%`);
            this._refreshLiveStats();
            if (pct >= 100) {
              this._status('Playing');
              this._hideOverlay();
            }
          },
        });
      } catch (e) {
        clearInterval(this._dlInterval);
        clearInterval(this._statsInterval);
        this._showError('Streaming error:\n' + e.message);
      }
    }

    /* ── Live stats shown in the overlay during buffering ─────────────── */
    _refreshLiveStats() {
      const t = this._torrent;
      if (!t) return;
      const peers = t.peers?.size ?? 0;
      const pct   = t.totalSize > 0 ? (t.downloaded / t.totalSize * 100).toFixed(1) : '0.0';
      const spd   = this._speed > 0 ? '↓ ' + fmt(this._speed) + '/s' : '';
      this._liveStats(
        [peers + ' peer' + (peers !== 1 ? 's' : ''), pct + '%', spd]
          .filter(Boolean).join(' · ')
      );
    }

    /* ── Console stats (every statsInterval ms) ──────────────────────── */
    _logStats(final = false) {
      const t = this._torrent;
      if (!t) return;

      const now     = Date.now();
      const elapsed = Math.max((now - this._lastStatsTime) / 1000, 0.001);
      this._speed        = Math.max(0, (t.downloaded - this._lastBytes) / elapsed);
      this._lastBytes    = t.downloaded;
      this._lastStatsTime = now;

      const pct    = t.totalSize > 0 ? (t.downloaded / t.totalSize * 100).toFixed(1) : '0.0';
      const pieces = t.pieces?.size    ?? 0;
      const total  = t.pieceCount      ?? 0;
      const peers  = t.peers?.size     ?? 0;
      const spd    = this._speed > 0 ? `  |  ↓ ${fmt(this._speed)}/s` : '';

      console.log(
        `[RtcTorrentPlayer] ${final ? '✓ Complete' : 'Stats'}:` +
        `  ${fmt(t.downloaded)} / ${fmt(t.totalSize)}  (${pct}%)` +
        `  |  ${pieces}/${total} pieces` +
        `  |  ${peers} peer${peers !== 1 ? 's' : ''}` +
        spd
      );

      this._refreshLiveStats();

      if (typeof this._o.onStats === 'function') {
        this._o.onStats({
          downloaded:  t.downloaded,
          total:       t.totalSize,
          percent:     parseFloat(pct),
          pieces,
          totalPieces: total,
          peers,
          speed:       this._speed,
        });
      }
    }

    /* ── Public: tear down ───────────────────────────────────────────── */
    destroy() {
      clearInterval(this._dlInterval);
      clearInterval(this._statsInterval);
      if (this._torrent) { this._torrent.active = false; this._torrent = null; }
      if (this._client)  { this._client.stop().catch(() => {}); this._client = null; }
      if (this._vjs)     { this._vjs.dispose(); this._vjs = null; }
      this._el.innerHTML = '';
    }
  }

  /* ── Public API ─────────────────────────────────────────────────────────── */
  function RtcTorrentPlayer(selector, opts) {
    return new RtcTorrentPlayerInstance(selector, opts);
  }
  RtcTorrentPlayer.Player = RtcTorrentPlayerInstance;

  global.RtcTorrentPlayer = RtcTorrentPlayer;

}(window));
