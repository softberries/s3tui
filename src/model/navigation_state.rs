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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_navigation_state_correctly() {
        let ns = NavigationState {
            current_bucket: Some("bucket".into()),
            current_prefix: Some("prefix".into()),
        };

        let res = NavigationState::new(Some("bucket".into()), Some("prefix".into()));

        assert_eq!(ns.current_bucket, res.current_bucket);
        assert_eq!(ns.current_prefix, res.current_prefix);
    }
}