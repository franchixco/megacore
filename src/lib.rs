
pub mod core;
pub mod crypto;
pub mod downloader;
pub mod http;
pub mod mega_api;

use crate::core::{Download, DownloadStatus};
use crate::downloader::manager::DownloadManager;
use std::sync::{Arc, Mutex};

pub struct MegaDownloader {
    download_manager: Arc<Mutex<DownloadManager>>,
}

impl MegaDownloader {
    pub fn new() -> Self {
        Self {
            download_manager: Arc::new(Mutex::new(DownloadManager::new())),
        }
    }

    pub fn add_download(&self, url: &str, download_path: &str) {
        let download = Download {
            url: url.to_string(),
            download_path: download_path.to_string(),
            file_metadata: None,
            progress: 0,
            status: DownloadStatus::Pending,
        };
        self.download_manager.lock().unwrap().add_download(download);
    }

    pub fn get_downloads(&self) -> Vec<Download> {
        self.download_manager.lock().unwrap().queue.clone().into_iter().collect()
    }

    pub fn pause_download(&self, url: &str) {
        self.download_manager.lock().unwrap().pause_download(url);
    }

    pub fn resume_download(&self, url: &str) {
        self.download_manager.lock().unwrap().resume_download(url);
    }

    pub fn cancel_download(&self, url: &str) {
        self.download_manager.lock().unwrap().cancel_download(url);
    }
}
