/// Keeps the information about the current progress of downloaded item
#[derive(Debug, Clone)]
pub struct DownloadProgressItem {
    pub progress: f64,
    pub bucket: String,
    pub name: String,
}