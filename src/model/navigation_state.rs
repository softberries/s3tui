#[derive(Clone, Debug)]
pub struct NavigationState {
    pub current_bucket: Option<String>,
    pub current_prefix: Option<String>,
}

impl NavigationState {
    pub fn new(bucket: Option<String>, prefix: Option<String>) -> Self {
        NavigationState {
            current_bucket: bucket,
            current_prefix: prefix,
        }
    }
}