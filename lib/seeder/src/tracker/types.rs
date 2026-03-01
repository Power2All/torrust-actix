use percent_encoding::{
    AsciiSet,
    CONTROLS
};

pub const TRACKER_ENCODE_SET: &AsciiSet = &CONTROLS
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
    .add(b'!')
    .add(b'$')
    .add(b'\'')
    .add(b'(')
    .add(b')')
    .add(b'*')
    .add(b',')
    .add(b';')
    .add(b':')
    .add(b'/')
    .add(b'~');