
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub name: String,
    pub size: u64,
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DownloadStatus {
    Pending,
    Downloading,
    Paused,
    Completed,
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Download {
    pub url: String,
    pub file_metadata: Option<FileMetadata>,
    pub progress: u64,
    pub status: DownloadStatus,
    pub download_path: String,
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub id: u64,
    pub start: u64,
    pub end: u64,
    pub data: Option<Vec<u8>>,
}
