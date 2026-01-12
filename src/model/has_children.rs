//! Trait and utilities for items that can have nested children

/// Trait for items that can contain child items of the same type
pub trait HasChildren: Clone {
    /// Returns a reference to the children, if any
    fn children(&self) -> Option<&Vec<Self>>;

    /// Returns owned children, or an empty vec if none
    fn take_children(self) -> Vec<Self>;

    /// Returns true if this item has no children (is a single file)
    fn is_leaf(&self) -> bool {
        self.children().is_none()
    }
}

/// Flattens a collection of items with children into a flat list.
/// Items with children are expanded, while leaf items are kept as-is.
pub fn flatten_items<T: HasChildren>(items: Vec<T>) -> Vec<T> {
    let nested: Vec<Vec<T>> = items
        .iter()
        .map(|i| i.clone().take_children())
        .collect();
    let mut children: Vec<T> = nested.into_iter().flatten().collect();
    let single_files: Vec<T> = items
        .into_iter()
        .filter(|i| i.is_leaf())
        .collect();
    children.extend(single_files);
    children
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    struct TestItem {
        name: String,
        children: Option<Vec<TestItem>>,
    }

    impl HasChildren for TestItem {
        fn children(&self) -> Option<&Vec<Self>> {
            self.children.as_ref()
        }

        fn take_children(self) -> Vec<Self> {
            self.children.unwrap_or_default()
        }
    }

    #[test]
    fn test_flatten_items_with_no_children() {
        let items = vec![
            TestItem {
                name: "file1".into(),
                children: None,
            },
            TestItem {
                name: "file2".into(),
                children: None,
            },
        ];
        let result = flatten_items(items.clone());
        assert_eq!(result.len(), 2);
        assert!(result.iter().any(|i| i.name == "file1"));
        assert!(result.iter().any(|i| i.name == "file2"));
    }

    #[test]
    fn test_flatten_items_with_children() {
        let child1 = TestItem {
            name: "child1".into(),
            children: None,
        };
        let child2 = TestItem {
            name: "child2".into(),
            children: None,
        };
        let parent = TestItem {
            name: "parent".into(),
            children: Some(vec![child1.clone(), child2.clone()]),
        };
        let single = TestItem {
            name: "single".into(),
            children: None,
        };

        let items = vec![parent, single.clone()];
        let result = flatten_items(items);

        // Should contain child1, child2, and single (parent is not included as a leaf)
        assert_eq!(result.len(), 3);
        assert!(result.iter().any(|i| i.name == "child1"));
        assert!(result.iter().any(|i| i.name == "child2"));
        assert!(result.iter().any(|i| i.name == "single"));
    }

    #[test]
    fn test_flatten_items_empty() {
        let items: Vec<TestItem> = vec![];
        let result = flatten_items(items);
        assert!(result.is_empty());
    }

    #[test]
    fn test_is_leaf() {
        let leaf = TestItem {
            name: "leaf".into(),
            children: None,
        };
        let parent = TestItem {
            name: "parent".into(),
            children: Some(vec![]),
        };

        assert!(leaf.is_leaf());
        assert!(!parent.is_leaf());
    }
}
