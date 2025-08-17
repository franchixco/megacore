
use crate::core::{Download, DownloadStatus};
use std::collections::VecDeque;

pub struct DownloadManager {
    pub queue: VecDeque<Download>,
}

impl DownloadManager {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    pub fn add_download(&mut self, download: Download) {
        self.queue.push_back(download);
    }

    pub fn get_next_download(&mut self) -> Option<Download> {
        self.queue.pop_front()
    }

    pub fn has_downloads(&self) -> bool {
        !self.queue.is_empty()
    }

    pub fn pause_download(&mut self, download_url: &str) {
        if let Some(download) = self
            .queue
            .iter_mut()
            .find(|d| d.url == download_url)
        {
            download.status = DownloadStatus::Paused;
        }
    }

    pub fn resume_download(&mut self, download_url: &str) {
        if let Some(download) = self
            .queue
            .iter_mut()
            .find(|d| d.url == download_url)
        {
            download.status = DownloadStatus::Pending;
        }
    }

    pub fn cancel_download(&mut self, download_url: &str) {
        self.queue.retain(|d| d.url != download_url);
    }
}
