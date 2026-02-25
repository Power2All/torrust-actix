#[allow(dead_code)]
pub type PieceHash = [u8; 20];
pub type InfoHash = [u8; 20];

use percent_encoding::{AsciiSet, CONTROLS};

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