use std::collections::BTreeMap;
use std::sync::Arc;
use parking_lot::RwLock;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::peer_id::PeerId;
use crate::tracker::structs::torrent_entry::TorrentEntry;
use crate::tracker::structs::torrent_sharding::TorrentSharding;

#[allow(dead_code)]
impl TorrentSharding {
    fn default() -> Self {
        Self::new()
    }

    #[tracing::instrument(level = "debug")]
    pub fn new() -> TorrentSharding
    {
        TorrentSharding {
            shard_000: Arc::new(RwLock::new(Default::default())),
            shard_001: Arc::new(RwLock::new(Default::default())),
            shard_002: Arc::new(RwLock::new(Default::default())),
            shard_003: Arc::new(RwLock::new(Default::default())),
            shard_004: Arc::new(RwLock::new(Default::default())),
            shard_005: Arc::new(RwLock::new(Default::default())),
            shard_006: Arc::new(RwLock::new(Default::default())),
            shard_007: Arc::new(RwLock::new(Default::default())),
            shard_008: Arc::new(RwLock::new(Default::default())),
            shard_009: Arc::new(RwLock::new(Default::default())),
            shard_010: Arc::new(RwLock::new(Default::default())),
            shard_011: Arc::new(RwLock::new(Default::default())),
            shard_012: Arc::new(RwLock::new(Default::default())),
            shard_013: Arc::new(RwLock::new(Default::default())),
            shard_014: Arc::new(RwLock::new(Default::default())),
            shard_015: Arc::new(RwLock::new(Default::default())),
            shard_016: Arc::new(RwLock::new(Default::default())),
            shard_017: Arc::new(RwLock::new(Default::default())),
            shard_018: Arc::new(RwLock::new(Default::default())),
            shard_019: Arc::new(RwLock::new(Default::default())),
            shard_020: Arc::new(RwLock::new(Default::default())),
            shard_021: Arc::new(RwLock::new(Default::default())),
            shard_022: Arc::new(RwLock::new(Default::default())),
            shard_023: Arc::new(RwLock::new(Default::default())),
            shard_024: Arc::new(RwLock::new(Default::default())),
            shard_025: Arc::new(RwLock::new(Default::default())),
            shard_026: Arc::new(RwLock::new(Default::default())),
            shard_027: Arc::new(RwLock::new(Default::default())),
            shard_028: Arc::new(RwLock::new(Default::default())),
            shard_029: Arc::new(RwLock::new(Default::default())),
            shard_030: Arc::new(RwLock::new(Default::default())),
            shard_031: Arc::new(RwLock::new(Default::default())),
            shard_032: Arc::new(RwLock::new(Default::default())),
            shard_033: Arc::new(RwLock::new(Default::default())),
            shard_034: Arc::new(RwLock::new(Default::default())),
            shard_035: Arc::new(RwLock::new(Default::default())),
            shard_036: Arc::new(RwLock::new(Default::default())),
            shard_037: Arc::new(RwLock::new(Default::default())),
            shard_038: Arc::new(RwLock::new(Default::default())),
            shard_039: Arc::new(RwLock::new(Default::default())),
            shard_040: Arc::new(RwLock::new(Default::default())),
            shard_041: Arc::new(RwLock::new(Default::default())),
            shard_042: Arc::new(RwLock::new(Default::default())),
            shard_043: Arc::new(RwLock::new(Default::default())),
            shard_044: Arc::new(RwLock::new(Default::default())),
            shard_045: Arc::new(RwLock::new(Default::default())),
            shard_046: Arc::new(RwLock::new(Default::default())),
            shard_047: Arc::new(RwLock::new(Default::default())),
            shard_048: Arc::new(RwLock::new(Default::default())),
            shard_049: Arc::new(RwLock::new(Default::default())),
            shard_050: Arc::new(RwLock::new(Default::default())),
            shard_051: Arc::new(RwLock::new(Default::default())),
            shard_052: Arc::new(RwLock::new(Default::default())),
            shard_053: Arc::new(RwLock::new(Default::default())),
            shard_054: Arc::new(RwLock::new(Default::default())),
            shard_055: Arc::new(RwLock::new(Default::default())),
            shard_056: Arc::new(RwLock::new(Default::default())),
            shard_057: Arc::new(RwLock::new(Default::default())),
            shard_058: Arc::new(RwLock::new(Default::default())),
            shard_059: Arc::new(RwLock::new(Default::default())),
            shard_060: Arc::new(RwLock::new(Default::default())),
            shard_061: Arc::new(RwLock::new(Default::default())),
            shard_062: Arc::new(RwLock::new(Default::default())),
            shard_063: Arc::new(RwLock::new(Default::default())),
            shard_064: Arc::new(RwLock::new(Default::default())),
            shard_065: Arc::new(RwLock::new(Default::default())),
            shard_066: Arc::new(RwLock::new(Default::default())),
            shard_067: Arc::new(RwLock::new(Default::default())),
            shard_068: Arc::new(RwLock::new(Default::default())),
            shard_069: Arc::new(RwLock::new(Default::default())),
            shard_070: Arc::new(RwLock::new(Default::default())),
            shard_071: Arc::new(RwLock::new(Default::default())),
            shard_072: Arc::new(RwLock::new(Default::default())),
            shard_073: Arc::new(RwLock::new(Default::default())),
            shard_074: Arc::new(RwLock::new(Default::default())),
            shard_075: Arc::new(RwLock::new(Default::default())),
            shard_076: Arc::new(RwLock::new(Default::default())),
            shard_077: Arc::new(RwLock::new(Default::default())),
            shard_078: Arc::new(RwLock::new(Default::default())),
            shard_079: Arc::new(RwLock::new(Default::default())),
            shard_080: Arc::new(RwLock::new(Default::default())),
            shard_081: Arc::new(RwLock::new(Default::default())),
            shard_082: Arc::new(RwLock::new(Default::default())),
            shard_083: Arc::new(RwLock::new(Default::default())),
            shard_084: Arc::new(RwLock::new(Default::default())),
            shard_085: Arc::new(RwLock::new(Default::default())),
            shard_086: Arc::new(RwLock::new(Default::default())),
            shard_087: Arc::new(RwLock::new(Default::default())),
            shard_088: Arc::new(RwLock::new(Default::default())),
            shard_089: Arc::new(RwLock::new(Default::default())),
            shard_090: Arc::new(RwLock::new(Default::default())),
            shard_091: Arc::new(RwLock::new(Default::default())),
            shard_092: Arc::new(RwLock::new(Default::default())),
            shard_093: Arc::new(RwLock::new(Default::default())),
            shard_094: Arc::new(RwLock::new(Default::default())),
            shard_095: Arc::new(RwLock::new(Default::default())),
            shard_096: Arc::new(RwLock::new(Default::default())),
            shard_097: Arc::new(RwLock::new(Default::default())),
            shard_098: Arc::new(RwLock::new(Default::default())),
            shard_099: Arc::new(RwLock::new(Default::default())),
            shard_100: Arc::new(RwLock::new(Default::default())),
            shard_101: Arc::new(RwLock::new(Default::default())),
            shard_102: Arc::new(RwLock::new(Default::default())),
            shard_103: Arc::new(RwLock::new(Default::default())),
            shard_104: Arc::new(RwLock::new(Default::default())),
            shard_105: Arc::new(RwLock::new(Default::default())),
            shard_106: Arc::new(RwLock::new(Default::default())),
            shard_107: Arc::new(RwLock::new(Default::default())),
            shard_108: Arc::new(RwLock::new(Default::default())),
            shard_109: Arc::new(RwLock::new(Default::default())),
            shard_110: Arc::new(RwLock::new(Default::default())),
            shard_111: Arc::new(RwLock::new(Default::default())),
            shard_112: Arc::new(RwLock::new(Default::default())),
            shard_113: Arc::new(RwLock::new(Default::default())),
            shard_114: Arc::new(RwLock::new(Default::default())),
            shard_115: Arc::new(RwLock::new(Default::default())),
            shard_116: Arc::new(RwLock::new(Default::default())),
            shard_117: Arc::new(RwLock::new(Default::default())),
            shard_118: Arc::new(RwLock::new(Default::default())),
            shard_119: Arc::new(RwLock::new(Default::default())),
            shard_120: Arc::new(RwLock::new(Default::default())),
            shard_121: Arc::new(RwLock::new(Default::default())),
            shard_122: Arc::new(RwLock::new(Default::default())),
            shard_123: Arc::new(RwLock::new(Default::default())),
            shard_124: Arc::new(RwLock::new(Default::default())),
            shard_125: Arc::new(RwLock::new(Default::default())),
            shard_126: Arc::new(RwLock::new(Default::default())),
            shard_127: Arc::new(RwLock::new(Default::default())),
            shard_128: Arc::new(RwLock::new(Default::default())),
            shard_129: Arc::new(RwLock::new(Default::default())),
            shard_130: Arc::new(RwLock::new(Default::default())),
            shard_131: Arc::new(RwLock::new(Default::default())),
            shard_132: Arc::new(RwLock::new(Default::default())),
            shard_133: Arc::new(RwLock::new(Default::default())),
            shard_134: Arc::new(RwLock::new(Default::default())),
            shard_135: Arc::new(RwLock::new(Default::default())),
            shard_136: Arc::new(RwLock::new(Default::default())),
            shard_137: Arc::new(RwLock::new(Default::default())),
            shard_138: Arc::new(RwLock::new(Default::default())),
            shard_139: Arc::new(RwLock::new(Default::default())),
            shard_140: Arc::new(RwLock::new(Default::default())),
            shard_141: Arc::new(RwLock::new(Default::default())),
            shard_142: Arc::new(RwLock::new(Default::default())),
            shard_143: Arc::new(RwLock::new(Default::default())),
            shard_144: Arc::new(RwLock::new(Default::default())),
            shard_145: Arc::new(RwLock::new(Default::default())),
            shard_146: Arc::new(RwLock::new(Default::default())),
            shard_147: Arc::new(RwLock::new(Default::default())),
            shard_148: Arc::new(RwLock::new(Default::default())),
            shard_149: Arc::new(RwLock::new(Default::default())),
            shard_150: Arc::new(RwLock::new(Default::default())),
            shard_151: Arc::new(RwLock::new(Default::default())),
            shard_152: Arc::new(RwLock::new(Default::default())),
            shard_153: Arc::new(RwLock::new(Default::default())),
            shard_154: Arc::new(RwLock::new(Default::default())),
            shard_155: Arc::new(RwLock::new(Default::default())),
            shard_156: Arc::new(RwLock::new(Default::default())),
            shard_157: Arc::new(RwLock::new(Default::default())),
            shard_158: Arc::new(RwLock::new(Default::default())),
            shard_159: Arc::new(RwLock::new(Default::default())),
            shard_160: Arc::new(RwLock::new(Default::default())),
            shard_161: Arc::new(RwLock::new(Default::default())),
            shard_162: Arc::new(RwLock::new(Default::default())),
            shard_163: Arc::new(RwLock::new(Default::default())),
            shard_164: Arc::new(RwLock::new(Default::default())),
            shard_165: Arc::new(RwLock::new(Default::default())),
            shard_166: Arc::new(RwLock::new(Default::default())),
            shard_167: Arc::new(RwLock::new(Default::default())),
            shard_168: Arc::new(RwLock::new(Default::default())),
            shard_169: Arc::new(RwLock::new(Default::default())),
            shard_170: Arc::new(RwLock::new(Default::default())),
            shard_171: Arc::new(RwLock::new(Default::default())),
            shard_172: Arc::new(RwLock::new(Default::default())),
            shard_173: Arc::new(RwLock::new(Default::default())),
            shard_174: Arc::new(RwLock::new(Default::default())),
            shard_175: Arc::new(RwLock::new(Default::default())),
            shard_176: Arc::new(RwLock::new(Default::default())),
            shard_177: Arc::new(RwLock::new(Default::default())),
            shard_178: Arc::new(RwLock::new(Default::default())),
            shard_179: Arc::new(RwLock::new(Default::default())),
            shard_180: Arc::new(RwLock::new(Default::default())),
            shard_181: Arc::new(RwLock::new(Default::default())),
            shard_182: Arc::new(RwLock::new(Default::default())),
            shard_183: Arc::new(RwLock::new(Default::default())),
            shard_184: Arc::new(RwLock::new(Default::default())),
            shard_185: Arc::new(RwLock::new(Default::default())),
            shard_186: Arc::new(RwLock::new(Default::default())),
            shard_187: Arc::new(RwLock::new(Default::default())),
            shard_188: Arc::new(RwLock::new(Default::default())),
            shard_189: Arc::new(RwLock::new(Default::default())),
            shard_190: Arc::new(RwLock::new(Default::default())),
            shard_191: Arc::new(RwLock::new(Default::default())),
            shard_192: Arc::new(RwLock::new(Default::default())),
            shard_193: Arc::new(RwLock::new(Default::default())),
            shard_194: Arc::new(RwLock::new(Default::default())),
            shard_195: Arc::new(RwLock::new(Default::default())),
            shard_196: Arc::new(RwLock::new(Default::default())),
            shard_197: Arc::new(RwLock::new(Default::default())),
            shard_198: Arc::new(RwLock::new(Default::default())),
            shard_199: Arc::new(RwLock::new(Default::default())),
            shard_200: Arc::new(RwLock::new(Default::default())),
            shard_201: Arc::new(RwLock::new(Default::default())),
            shard_202: Arc::new(RwLock::new(Default::default())),
            shard_203: Arc::new(RwLock::new(Default::default())),
            shard_204: Arc::new(RwLock::new(Default::default())),
            shard_205: Arc::new(RwLock::new(Default::default())),
            shard_206: Arc::new(RwLock::new(Default::default())),
            shard_207: Arc::new(RwLock::new(Default::default())),
            shard_208: Arc::new(RwLock::new(Default::default())),
            shard_209: Arc::new(RwLock::new(Default::default())),
            shard_210: Arc::new(RwLock::new(Default::default())),
            shard_211: Arc::new(RwLock::new(Default::default())),
            shard_212: Arc::new(RwLock::new(Default::default())),
            shard_213: Arc::new(RwLock::new(Default::default())),
            shard_214: Arc::new(RwLock::new(Default::default())),
            shard_215: Arc::new(RwLock::new(Default::default())),
            shard_216: Arc::new(RwLock::new(Default::default())),
            shard_217: Arc::new(RwLock::new(Default::default())),
            shard_218: Arc::new(RwLock::new(Default::default())),
            shard_219: Arc::new(RwLock::new(Default::default())),
            shard_220: Arc::new(RwLock::new(Default::default())),
            shard_221: Arc::new(RwLock::new(Default::default())),
            shard_222: Arc::new(RwLock::new(Default::default())),
            shard_223: Arc::new(RwLock::new(Default::default())),
            shard_224: Arc::new(RwLock::new(Default::default())),
            shard_225: Arc::new(RwLock::new(Default::default())),
            shard_226: Arc::new(RwLock::new(Default::default())),
            shard_227: Arc::new(RwLock::new(Default::default())),
            shard_228: Arc::new(RwLock::new(Default::default())),
            shard_229: Arc::new(RwLock::new(Default::default())),
            shard_230: Arc::new(RwLock::new(Default::default())),
            shard_231: Arc::new(RwLock::new(Default::default())),
            shard_232: Arc::new(RwLock::new(Default::default())),
            shard_233: Arc::new(RwLock::new(Default::default())),
            shard_234: Arc::new(RwLock::new(Default::default())),
            shard_235: Arc::new(RwLock::new(Default::default())),
            shard_236: Arc::new(RwLock::new(Default::default())),
            shard_237: Arc::new(RwLock::new(Default::default())),
            shard_238: Arc::new(RwLock::new(Default::default())),
            shard_239: Arc::new(RwLock::new(Default::default())),
            shard_240: Arc::new(RwLock::new(Default::default())),
            shard_241: Arc::new(RwLock::new(Default::default())),
            shard_242: Arc::new(RwLock::new(Default::default())),
            shard_243: Arc::new(RwLock::new(Default::default())),
            shard_244: Arc::new(RwLock::new(Default::default())),
            shard_245: Arc::new(RwLock::new(Default::default())),
            shard_246: Arc::new(RwLock::new(Default::default())),
            shard_247: Arc::new(RwLock::new(Default::default())),
            shard_248: Arc::new(RwLock::new(Default::default())),
            shard_249: Arc::new(RwLock::new(Default::default())),
            shard_250: Arc::new(RwLock::new(Default::default())),
            shard_251: Arc::new(RwLock::new(Default::default())),
            shard_252: Arc::new(RwLock::new(Default::default())),
            shard_253: Arc::new(RwLock::new(Default::default())),
            shard_254: Arc::new(RwLock::new(Default::default())),
            shard_255: Arc::new(RwLock::new(Default::default())),
        }
    }

    #[tracing::instrument(level = "debug")]
    pub fn contains_torrent(&self, info_hash: InfoHash) -> bool
    {
        self.get_shard_content(info_hash.0[0]).contains_key(&info_hash)
    }

    #[tracing::instrument(level = "debug")]
    pub fn contains_peer(&self, info_hash: InfoHash, peer_id: PeerId) -> bool
    {
        match self.get_shard_content(info_hash.0[0]).get(&info_hash) {
            None => { false }
            Some(torrent_entry) => {
                if torrent_entry.seeds.contains_key(&peer_id) || torrent_entry.peers.contains_key(&peer_id) {
                    return true;
                }
                false
            }
        }
    }

    #[tracing::instrument(level = "debug")]
    #[allow(unreachable_patterns)]
    pub fn get_shard(&self, shard: u8) -> Option<Arc<RwLock<BTreeMap<InfoHash, TorrentEntry>>>>
    {
        match shard {
            0 => { Some(self.shard_000.clone()) }
            1 => { Some(self.shard_001.clone()) }
            2 => { Some(self.shard_002.clone()) }
            3 => { Some(self.shard_003.clone()) }
            4 => { Some(self.shard_004.clone()) }
            5 => { Some(self.shard_005.clone()) }
            6 => { Some(self.shard_006.clone()) }
            7 => { Some(self.shard_007.clone()) }
            8 => { Some(self.shard_008.clone()) }
            9 => { Some(self.shard_009.clone()) }
            10 => { Some(self.shard_010.clone()) }
            11 => { Some(self.shard_011.clone()) }
            12 => { Some(self.shard_012.clone()) }
            13 => { Some(self.shard_013.clone()) }
            14 => { Some(self.shard_014.clone()) }
            15 => { Some(self.shard_015.clone()) }
            16 => { Some(self.shard_016.clone()) }
            17 => { Some(self.shard_017.clone()) }
            18 => { Some(self.shard_018.clone()) }
            19 => { Some(self.shard_019.clone()) }
            20 => { Some(self.shard_020.clone()) }
            21 => { Some(self.shard_021.clone()) }
            22 => { Some(self.shard_022.clone()) }
            23 => { Some(self.shard_023.clone()) }
            24 => { Some(self.shard_024.clone()) }
            25 => { Some(self.shard_025.clone()) }
            26 => { Some(self.shard_026.clone()) }
            27 => { Some(self.shard_027.clone()) }
            28 => { Some(self.shard_028.clone()) }
            29 => { Some(self.shard_029.clone()) }
            30 => { Some(self.shard_030.clone()) }
            31 => { Some(self.shard_031.clone()) }
            32 => { Some(self.shard_032.clone()) }
            33 => { Some(self.shard_033.clone()) }
            34 => { Some(self.shard_034.clone()) }
            35 => { Some(self.shard_035.clone()) }
            36 => { Some(self.shard_036.clone()) }
            37 => { Some(self.shard_037.clone()) }
            38 => { Some(self.shard_038.clone()) }
            39 => { Some(self.shard_039.clone()) }
            40 => { Some(self.shard_040.clone()) }
            41 => { Some(self.shard_041.clone()) }
            42 => { Some(self.shard_042.clone()) }
            43 => { Some(self.shard_043.clone()) }
            44 => { Some(self.shard_044.clone()) }
            45 => { Some(self.shard_045.clone()) }
            46 => { Some(self.shard_046.clone()) }
            47 => { Some(self.shard_047.clone()) }
            48 => { Some(self.shard_048.clone()) }
            49 => { Some(self.shard_049.clone()) }
            50 => { Some(self.shard_050.clone()) }
            51 => { Some(self.shard_051.clone()) }
            52 => { Some(self.shard_052.clone()) }
            53 => { Some(self.shard_053.clone()) }
            54 => { Some(self.shard_054.clone()) }
            55 => { Some(self.shard_055.clone()) }
            56 => { Some(self.shard_056.clone()) }
            57 => { Some(self.shard_057.clone()) }
            58 => { Some(self.shard_058.clone()) }
            59 => { Some(self.shard_059.clone()) }
            60 => { Some(self.shard_060.clone()) }
            61 => { Some(self.shard_061.clone()) }
            62 => { Some(self.shard_062.clone()) }
            63 => { Some(self.shard_063.clone()) }
            64 => { Some(self.shard_064.clone()) }
            65 => { Some(self.shard_065.clone()) }
            66 => { Some(self.shard_066.clone()) }
            67 => { Some(self.shard_067.clone()) }
            68 => { Some(self.shard_068.clone()) }
            69 => { Some(self.shard_069.clone()) }
            70 => { Some(self.shard_070.clone()) }
            71 => { Some(self.shard_071.clone()) }
            72 => { Some(self.shard_072.clone()) }
            73 => { Some(self.shard_073.clone()) }
            74 => { Some(self.shard_074.clone()) }
            75 => { Some(self.shard_075.clone()) }
            76 => { Some(self.shard_076.clone()) }
            77 => { Some(self.shard_077.clone()) }
            78 => { Some(self.shard_078.clone()) }
            79 => { Some(self.shard_079.clone()) }
            80 => { Some(self.shard_080.clone()) }
            81 => { Some(self.shard_081.clone()) }
            82 => { Some(self.shard_082.clone()) }
            83 => { Some(self.shard_083.clone()) }
            84 => { Some(self.shard_084.clone()) }
            85 => { Some(self.shard_085.clone()) }
            86 => { Some(self.shard_086.clone()) }
            87 => { Some(self.shard_087.clone()) }
            88 => { Some(self.shard_088.clone()) }
            89 => { Some(self.shard_089.clone()) }
            90 => { Some(self.shard_090.clone()) }
            91 => { Some(self.shard_091.clone()) }
            92 => { Some(self.shard_092.clone()) }
            93 => { Some(self.shard_093.clone()) }
            94 => { Some(self.shard_094.clone()) }
            95 => { Some(self.shard_095.clone()) }
            96 => { Some(self.shard_096.clone()) }
            97 => { Some(self.shard_097.clone()) }
            98 => { Some(self.shard_098.clone()) }
            99 => { Some(self.shard_099.clone()) }
            100 => { Some(self.shard_100.clone()) }
            101 => { Some(self.shard_101.clone()) }
            102 => { Some(self.shard_102.clone()) }
            103 => { Some(self.shard_103.clone()) }
            104 => { Some(self.shard_104.clone()) }
            105 => { Some(self.shard_105.clone()) }
            106 => { Some(self.shard_106.clone()) }
            107 => { Some(self.shard_107.clone()) }
            108 => { Some(self.shard_108.clone()) }
            109 => { Some(self.shard_109.clone()) }
            110 => { Some(self.shard_110.clone()) }
            111 => { Some(self.shard_111.clone()) }
            112 => { Some(self.shard_112.clone()) }
            113 => { Some(self.shard_113.clone()) }
            114 => { Some(self.shard_114.clone()) }
            115 => { Some(self.shard_115.clone()) }
            116 => { Some(self.shard_116.clone()) }
            117 => { Some(self.shard_117.clone()) }
            118 => { Some(self.shard_118.clone()) }
            119 => { Some(self.shard_119.clone()) }
            120 => { Some(self.shard_120.clone()) }
            121 => { Some(self.shard_121.clone()) }
            122 => { Some(self.shard_122.clone()) }
            123 => { Some(self.shard_123.clone()) }
            124 => { Some(self.shard_124.clone()) }
            125 => { Some(self.shard_125.clone()) }
            126 => { Some(self.shard_126.clone()) }
            127 => { Some(self.shard_127.clone()) }
            128 => { Some(self.shard_128.clone()) }
            129 => { Some(self.shard_129.clone()) }
            130 => { Some(self.shard_130.clone()) }
            131 => { Some(self.shard_131.clone()) }
            132 => { Some(self.shard_132.clone()) }
            133 => { Some(self.shard_133.clone()) }
            134 => { Some(self.shard_134.clone()) }
            135 => { Some(self.shard_135.clone()) }
            136 => { Some(self.shard_136.clone()) }
            137 => { Some(self.shard_137.clone()) }
            138 => { Some(self.shard_138.clone()) }
            139 => { Some(self.shard_139.clone()) }
            140 => { Some(self.shard_140.clone()) }
            141 => { Some(self.shard_141.clone()) }
            142 => { Some(self.shard_142.clone()) }
            143 => { Some(self.shard_143.clone()) }
            144 => { Some(self.shard_144.clone()) }
            145 => { Some(self.shard_145.clone()) }
            146 => { Some(self.shard_146.clone()) }
            147 => { Some(self.shard_147.clone()) }
            148 => { Some(self.shard_148.clone()) }
            149 => { Some(self.shard_149.clone()) }
            150 => { Some(self.shard_150.clone()) }
            151 => { Some(self.shard_151.clone()) }
            152 => { Some(self.shard_152.clone()) }
            153 => { Some(self.shard_153.clone()) }
            154 => { Some(self.shard_154.clone()) }
            155 => { Some(self.shard_155.clone()) }
            156 => { Some(self.shard_156.clone()) }
            157 => { Some(self.shard_157.clone()) }
            158 => { Some(self.shard_158.clone()) }
            159 => { Some(self.shard_159.clone()) }
            160 => { Some(self.shard_160.clone()) }
            161 => { Some(self.shard_161.clone()) }
            162 => { Some(self.shard_162.clone()) }
            163 => { Some(self.shard_163.clone()) }
            164 => { Some(self.shard_164.clone()) }
            165 => { Some(self.shard_165.clone()) }
            166 => { Some(self.shard_166.clone()) }
            167 => { Some(self.shard_167.clone()) }
            168 => { Some(self.shard_168.clone()) }
            169 => { Some(self.shard_169.clone()) }
            170 => { Some(self.shard_170.clone()) }
            171 => { Some(self.shard_171.clone()) }
            172 => { Some(self.shard_172.clone()) }
            173 => { Some(self.shard_173.clone()) }
            174 => { Some(self.shard_174.clone()) }
            175 => { Some(self.shard_175.clone()) }
            176 => { Some(self.shard_176.clone()) }
            177 => { Some(self.shard_177.clone()) }
            178 => { Some(self.shard_178.clone()) }
            179 => { Some(self.shard_179.clone()) }
            180 => { Some(self.shard_180.clone()) }
            181 => { Some(self.shard_181.clone()) }
            182 => { Some(self.shard_182.clone()) }
            183 => { Some(self.shard_183.clone()) }
            184 => { Some(self.shard_184.clone()) }
            185 => { Some(self.shard_185.clone()) }
            186 => { Some(self.shard_186.clone()) }
            187 => { Some(self.shard_187.clone()) }
            188 => { Some(self.shard_188.clone()) }
            189 => { Some(self.shard_189.clone()) }
            190 => { Some(self.shard_190.clone()) }
            191 => { Some(self.shard_191.clone()) }
            192 => { Some(self.shard_192.clone()) }
            193 => { Some(self.shard_193.clone()) }
            194 => { Some(self.shard_194.clone()) }
            195 => { Some(self.shard_195.clone()) }
            196 => { Some(self.shard_196.clone()) }
            197 => { Some(self.shard_197.clone()) }
            198 => { Some(self.shard_198.clone()) }
            199 => { Some(self.shard_199.clone()) }
            200 => { Some(self.shard_200.clone()) }
            201 => { Some(self.shard_201.clone()) }
            202 => { Some(self.shard_202.clone()) }
            203 => { Some(self.shard_203.clone()) }
            204 => { Some(self.shard_204.clone()) }
            205 => { Some(self.shard_205.clone()) }
            206 => { Some(self.shard_206.clone()) }
            207 => { Some(self.shard_207.clone()) }
            208 => { Some(self.shard_208.clone()) }
            209 => { Some(self.shard_209.clone()) }
            210 => { Some(self.shard_210.clone()) }
            211 => { Some(self.shard_211.clone()) }
            212 => { Some(self.shard_212.clone()) }
            213 => { Some(self.shard_213.clone()) }
            214 => { Some(self.shard_214.clone()) }
            215 => { Some(self.shard_215.clone()) }
            216 => { Some(self.shard_216.clone()) }
            217 => { Some(self.shard_217.clone()) }
            218 => { Some(self.shard_218.clone()) }
            219 => { Some(self.shard_219.clone()) }
            220 => { Some(self.shard_220.clone()) }
            221 => { Some(self.shard_221.clone()) }
            222 => { Some(self.shard_222.clone()) }
            223 => { Some(self.shard_223.clone()) }
            224 => { Some(self.shard_224.clone()) }
            225 => { Some(self.shard_225.clone()) }
            226 => { Some(self.shard_226.clone()) }
            227 => { Some(self.shard_227.clone()) }
            228 => { Some(self.shard_228.clone()) }
            229 => { Some(self.shard_229.clone()) }
            230 => { Some(self.shard_230.clone()) }
            231 => { Some(self.shard_231.clone()) }
            232 => { Some(self.shard_232.clone()) }
            233 => { Some(self.shard_233.clone()) }
            234 => { Some(self.shard_234.clone()) }
            235 => { Some(self.shard_235.clone()) }
            236 => { Some(self.shard_236.clone()) }
            237 => { Some(self.shard_237.clone()) }
            238 => { Some(self.shard_238.clone()) }
            239 => { Some(self.shard_239.clone()) }
            240 => { Some(self.shard_240.clone()) }
            241 => { Some(self.shard_241.clone()) }
            242 => { Some(self.shard_242.clone()) }
            243 => { Some(self.shard_243.clone()) }
            244 => { Some(self.shard_244.clone()) }
            245 => { Some(self.shard_245.clone()) }
            246 => { Some(self.shard_246.clone()) }
            247 => { Some(self.shard_247.clone()) }
            248 => { Some(self.shard_248.clone()) }
            249 => { Some(self.shard_249.clone()) }
            250 => { Some(self.shard_250.clone()) }
            251 => { Some(self.shard_251.clone()) }
            252 => { Some(self.shard_252.clone()) }
            253 => { Some(self.shard_253.clone()) }
            254 => { Some(self.shard_254.clone()) }
            255 => { Some(self.shard_255.clone()) }
            _ => { None }
        }
    }

    #[tracing::instrument(level = "debug")]
    pub fn get_shard_content(&self, shard: u8) -> BTreeMap<InfoHash, TorrentEntry>
    {
        self.get_shard(shard).unwrap().read_recursive().clone()
    }

    #[tracing::instrument(level = "debug")]
    pub fn get_all_content(&self) -> BTreeMap<InfoHash, TorrentEntry>
    {
        let mut torrents_return = BTreeMap::new();
        for index in 0u8..=255u8 {
            let mut shard = self.get_shard(index).unwrap().read_recursive().clone();
            torrents_return.append(&mut shard);
        }
        torrents_return
    }

    #[tracing::instrument(level = "debug")]
    pub fn get_torrents_amount(&self) -> u64
    {
        let mut torrents = 0u64;
        for index in 0u8..=255u8 {
            torrents += self.get_shard(index).unwrap().read_recursive().len() as u64;
        }
        torrents
    }
}