#[derive(Debug, Clone)]
pub struct DownloadProgressItem {
    pub progress: f64,
    pub bucket: String,
    pub name: String,
}