#[derive(Debug, Clone, PartialEq, Default)]
pub enum TorrentVersion {
    #[default]
    V1,
    V2,
    Hybrid,
}