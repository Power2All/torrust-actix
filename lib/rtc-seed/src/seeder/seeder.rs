use rand::RngExt;

pub fn generate_peer_id() -> [u8; 20] {
    let prefix = b"-RS1000-";
    let mut id = [0u8; 20];
    id[..prefix.len()].copy_from_slice(prefix);
    rand::rng().fill(&mut id[prefix.len()..]);
    for b in &mut id[prefix.len()..] {
        *b = b'0' + (*b % 10);
    }
    id
}

pub fn fmt_bytes(n: u64) -> String {
    if n < 1024 {
        format!("{} B", n)
    } else if n < 1024 * 1024 {
        format!("{:.1} KB", n as f64 / 1024.0)
    } else if n < 1024 * 1024 * 1024 {
        format!("{:.1} MB", n as f64 / 1024.0 / 1024.0)
    } else {
        format!("{:.2} GB", n as f64 / 1024.0 / 1024.0 / 1024.0)
    }
}