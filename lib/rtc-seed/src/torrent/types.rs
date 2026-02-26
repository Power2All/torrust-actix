use percent_encoding::{
    AsciiSet,
    CONTROLS
};

#[allow(dead_code)]
pub type PieceHash = [u8; 20];
pub type InfoHash = [u8; 20];
pub type V2InfoHash = [u8; 32];

pub const QUERY_ENCODE: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'%')
    .add(b'<')
    .add(b'>')
    .add(b'[')
    .add(b'\\')
    .add(b']')
    .add(b'^')
    .add(b'`')
    .add(b'{')
    .add(b'|')
    .add(b'}')
    .add(b'?')
    .add(b'&')
    .add(b'=')
    .add(b'+')
    .add(b'@')
    .add(b':')
    .add(b'/');