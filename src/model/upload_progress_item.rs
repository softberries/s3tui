/// Keeps the state of the currently uploaded items
#[derive(Debug, Clone)]
pub struct UploadProgressItem {
    pub progress: f64,
    pub uri: String,
}
