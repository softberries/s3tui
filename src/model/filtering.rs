//! Filtering types and utilities for file lists

/// Trait for items that can be filtered by name
pub trait Filterable {
    /// Get the name of the item for filtering
    fn filterable_name(&self) -> &str;
}

/// Filter items by a search query (case-insensitive partial match)
pub fn filter_items<'a, T: Filterable>(items: &'a [T], query: &str) -> Vec<&'a T> {
    if query.is_empty() {
        return items.iter().collect();
    }
    let query_lower = query.to_lowercase();
    items
        .iter()
        .filter(|item| item.filterable_name().to_lowercase().contains(&query_lower))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestItem {
        name: String,
    }

    impl Filterable for TestItem {
        fn filterable_name(&self) -> &str {
            &self.name
        }
    }

    #[test]
    fn test_filter_by_partial_name() {
        let items = vec![
            TestItem { name: "document.txt".to_string() },
            TestItem { name: "image.png".to_string() },
            TestItem { name: "docs_folder".to_string() },
        ];

        let result = filter_items(&items, "doc");
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "document.txt");
        assert_eq!(result[1].name, "docs_folder");
    }

    #[test]
    fn test_filter_case_insensitive() {
        let items = vec![
            TestItem { name: "Document.TXT".to_string() },
            TestItem { name: "IMAGE.PNG".to_string() },
            TestItem { name: "readme.md".to_string() },
        ];

        let result = filter_items(&items, "DOC");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "Document.TXT");

        let result2 = filter_items(&items, "readme");
        assert_eq!(result2.len(), 1);
        assert_eq!(result2[0].name, "readme.md");
    }

    #[test]
    fn test_filter_empty_query_returns_all() {
        let items = vec![
            TestItem { name: "file1.txt".to_string() },
            TestItem { name: "file2.txt".to_string() },
        ];

        let result = filter_items(&items, "");
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_filter_no_matches_returns_empty() {
        let items = vec![
            TestItem { name: "file1.txt".to_string() },
            TestItem { name: "file2.txt".to_string() },
        ];

        let result = filter_items(&items, "xyz");
        assert!(result.is_empty());
    }
}
