use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub name: Vec<String>,
    pub length: u64,
    pub offset: u64,
}