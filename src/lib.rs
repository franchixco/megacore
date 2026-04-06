
pub mod core;
pub mod crypto;
pub mod downloader;
pub mod http;
pub mod mega_api;

use anyhow::Result;
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

    pub fn add_download(&self, url: &str, download_path: &str) -> Result<()> {
        let download = Download {
            url: url.to_string(),
            download_path: download_path.to_string(),
            file_metadata: None,
            progress: 0,
            status: DownloadStatus::Pending,
        };
        self.download_manager
            .lock()
            .map_err(|e| anyhow::anyhow!("Mutex poisoned: {}", e))?
            .add_download(download);
        Ok(())
    }

    pub fn get_downloads(&self) -> Result<Vec<Download>> {
        Ok(self
            .download_manager
            .lock()
            .map_err(|e| anyhow::anyhow!("Mutex poisoned: {}", e))?
            .queue
            .clone()
            .into_iter()
            .collect())
    }

    pub fn pause_download(&self, url: &str) -> Result<()> {
        self.download_manager
            .lock()
            .map_err(|e| anyhow::anyhow!("Mutex poisoned: {}", e))?
            .pause_download(url);
        Ok(())
    }

    pub fn resume_download(&self, url: &str) -> Result<()> {
        self.download_manager
            .lock()
            .map_err(|e| anyhow::anyhow!("Mutex poisoned: {}", e))?
            .resume_download(url);
        Ok(())
    }

    pub fn cancel_download(&self, url: &str) -> Result<()> {
        self.download_manager
            .lock()
            .map_err(|e| anyhow::anyhow!("Mutex poisoned: {}", e))?
            .cancel_download(url);
        Ok(())
    }
}
