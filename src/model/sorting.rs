//! Sorting types and utilities for file lists

/// Column to sort by
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortColumn {
    #[default]
    Name,
    Size,
    Type,
}

/// Sort direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortDirection {
    #[default]
    Ascending,
    Descending,
}

impl SortDirection {
    /// Toggle the sort direction
    pub fn toggle(&self) -> Self {
        match self {
            SortDirection::Ascending => SortDirection::Descending,
            SortDirection::Descending => SortDirection::Ascending,
        }
    }
}

/// Current sort state for a panel
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SortState {
    pub column: SortColumn,
    pub direction: SortDirection,
}

impl SortState {
    #[cfg(test)]
    pub fn new(column: SortColumn, direction: SortDirection) -> Self {
        Self { column, direction }
    }

    /// Set the sort column. If same column, toggle direction; if different, reset to ascending.
    pub fn set_column(&mut self, column: SortColumn) {
        if self.column == column {
            self.direction = self.direction.toggle();
        } else {
            self.column = column;
            self.direction = SortDirection::Ascending;
        }
    }

    /// Get sort indicator character for a column
    pub fn indicator(&self, column: SortColumn) -> &'static str {
        if self.column == column {
            match self.direction {
                SortDirection::Ascending => " ^",
                SortDirection::Descending => " v",
            }
        } else {
            ""
        }
    }
}

/// Trait for items that can be sorted
pub trait Sortable {
    fn name(&self) -> &str;
    fn size_bytes(&self) -> u64;
    fn file_type(&self) -> &str;
    fn is_directory(&self) -> bool;
}

/// Parse a human-readable size string to bytes for comparison
/// Handles formats like "1.5 MB", "100 KB", "2 GB", etc.
pub fn parse_size_to_bytes(size: &str) -> u64 {
    let size = size.trim();
    if size.is_empty() || size == "-" {
        return 0;
    }

    let parts: Vec<&str> = size.split_whitespace().collect();
    if parts.is_empty() {
        return 0;
    }

    let number: f64 = parts[0].parse().unwrap_or(0.0);
    let unit = parts.get(1).unwrap_or(&"B").to_uppercase();

    let multiplier: u64 = match unit.as_str() {
        "B" => 1,
        "KB" => 1024,
        "MB" => 1024 * 1024,
        "GB" => 1024 * 1024 * 1024,
        "TB" => 1024 * 1024 * 1024 * 1024,
        _ => 1,
    };

    (number * multiplier as f64) as u64
}

/// Sort a slice of sortable items in place
pub fn sort_items<T: Sortable>(items: &mut [T], state: &SortState) {
    // Directories always come first, then sort within each group
    items.sort_by(|a, b| {
        // Directories first
        match (a.is_directory(), b.is_directory()) {
            (true, false) => return std::cmp::Ordering::Less,
            (false, true) => return std::cmp::Ordering::Greater,
            _ => {}
        }

        let ordering = match state.column {
            SortColumn::Name => a.name().to_lowercase().cmp(&b.name().to_lowercase()),
            SortColumn::Size => a.size_bytes().cmp(&b.size_bytes()),
            SortColumn::Type => a.file_type().to_lowercase().cmp(&b.file_type().to_lowercase()),
        };

        match state.direction {
            SortDirection::Ascending => ordering,
            SortDirection::Descending => ordering.reverse(),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone)]
    struct TestItem {
        name: String,
        size: String,
        file_type: String,
        is_dir: bool,
    }

    impl Sortable for TestItem {
        fn name(&self) -> &str {
            &self.name
        }
        fn size_bytes(&self) -> u64 {
            parse_size_to_bytes(&self.size)
        }
        fn file_type(&self) -> &str {
            &self.file_type
        }
        fn is_directory(&self) -> bool {
            self.is_dir
        }
    }

    #[test]
    fn test_sort_direction_toggle() {
        assert_eq!(SortDirection::Ascending.toggle(), SortDirection::Descending);
        assert_eq!(SortDirection::Descending.toggle(), SortDirection::Ascending);
    }

    #[test]
    fn test_sort_state_set_column_same() {
        let mut state = SortState::new(SortColumn::Name, SortDirection::Ascending);
        state.set_column(SortColumn::Name);
        assert_eq!(state.column, SortColumn::Name);
        assert_eq!(state.direction, SortDirection::Descending);
    }

    #[test]
    fn test_sort_state_set_column_different() {
        let mut state = SortState::new(SortColumn::Name, SortDirection::Descending);
        state.set_column(SortColumn::Size);
        assert_eq!(state.column, SortColumn::Size);
        assert_eq!(state.direction, SortDirection::Ascending);
    }

    #[test]
    fn test_sort_state_indicator() {
        let state = SortState::new(SortColumn::Name, SortDirection::Ascending);
        assert_eq!(state.indicator(SortColumn::Name), " ^");
        assert_eq!(state.indicator(SortColumn::Size), "");

        let state = SortState::new(SortColumn::Size, SortDirection::Descending);
        assert_eq!(state.indicator(SortColumn::Size), " v");
        assert_eq!(state.indicator(SortColumn::Name), "");
    }

    #[test]
    fn test_parse_size_to_bytes() {
        assert_eq!(parse_size_to_bytes("100 B"), 100);
        assert_eq!(parse_size_to_bytes("1 KB"), 1024);
        assert_eq!(parse_size_to_bytes("1.5 KB"), 1536);
        assert_eq!(parse_size_to_bytes("1 MB"), 1048576);
        assert_eq!(parse_size_to_bytes("1 GB"), 1073741824);
        assert_eq!(parse_size_to_bytes("-"), 0);
        assert_eq!(parse_size_to_bytes(""), 0);
    }

    #[test]
    fn test_sort_by_name_ascending() {
        let mut items = vec![
            TestItem { name: "zebra.txt".into(), size: "1 KB".into(), file_type: "txt".into(), is_dir: false },
            TestItem { name: "apple.txt".into(), size: "2 KB".into(), file_type: "txt".into(), is_dir: false },
            TestItem { name: "mango.txt".into(), size: "3 KB".into(), file_type: "txt".into(), is_dir: false },
        ];
        let state = SortState::new(SortColumn::Name, SortDirection::Ascending);
        sort_items(&mut items, &state);
        assert_eq!(items[0].name, "apple.txt");
        assert_eq!(items[1].name, "mango.txt");
        assert_eq!(items[2].name, "zebra.txt");
    }

    #[test]
    fn test_sort_by_name_descending() {
        let mut items = vec![
            TestItem { name: "apple.txt".into(), size: "1 KB".into(), file_type: "txt".into(), is_dir: false },
            TestItem { name: "zebra.txt".into(), size: "2 KB".into(), file_type: "txt".into(), is_dir: false },
            TestItem { name: "mango.txt".into(), size: "3 KB".into(), file_type: "txt".into(), is_dir: false },
        ];
        let state = SortState::new(SortColumn::Name, SortDirection::Descending);
        sort_items(&mut items, &state);
        assert_eq!(items[0].name, "zebra.txt");
        assert_eq!(items[1].name, "mango.txt");
        assert_eq!(items[2].name, "apple.txt");
    }

    #[test]
    fn test_sort_by_size_ascending() {
        let mut items = vec![
            TestItem { name: "large.txt".into(), size: "1 MB".into(), file_type: "txt".into(), is_dir: false },
            TestItem { name: "small.txt".into(), size: "1 KB".into(), file_type: "txt".into(), is_dir: false },
            TestItem { name: "medium.txt".into(), size: "100 KB".into(), file_type: "txt".into(), is_dir: false },
        ];
        let state = SortState::new(SortColumn::Size, SortDirection::Ascending);
        sort_items(&mut items, &state);
        assert_eq!(items[0].name, "small.txt");
        assert_eq!(items[1].name, "medium.txt");
        assert_eq!(items[2].name, "large.txt");
    }

    #[test]
    fn test_sort_by_size_descending() {
        let mut items = vec![
            TestItem { name: "small.txt".into(), size: "1 KB".into(), file_type: "txt".into(), is_dir: false },
            TestItem { name: "large.txt".into(), size: "1 MB".into(), file_type: "txt".into(), is_dir: false },
            TestItem { name: "medium.txt".into(), size: "100 KB".into(), file_type: "txt".into(), is_dir: false },
        ];
        let state = SortState::new(SortColumn::Size, SortDirection::Descending);
        sort_items(&mut items, &state);
        assert_eq!(items[0].name, "large.txt");
        assert_eq!(items[1].name, "medium.txt");
        assert_eq!(items[2].name, "small.txt");
    }

    #[test]
    fn test_sort_directories_first() {
        let mut items = vec![
            TestItem { name: "file.txt".into(), size: "1 KB".into(), file_type: "txt".into(), is_dir: false },
            TestItem { name: "dir".into(), size: "-".into(), file_type: "dir".into(), is_dir: true },
            TestItem { name: "another.txt".into(), size: "2 KB".into(), file_type: "txt".into(), is_dir: false },
        ];
        let state = SortState::new(SortColumn::Name, SortDirection::Ascending);
        sort_items(&mut items, &state);
        assert!(items[0].is_dir);
        assert!(!items[1].is_dir);
        assert!(!items[2].is_dir);
    }

    #[test]
    fn test_sort_by_type() {
        let mut items = vec![
            TestItem { name: "doc.pdf".into(), size: "1 KB".into(), file_type: "pdf".into(), is_dir: false },
            TestItem { name: "script.rs".into(), size: "2 KB".into(), file_type: "rs".into(), is_dir: false },
            TestItem { name: "readme.md".into(), size: "3 KB".into(), file_type: "md".into(), is_dir: false },
        ];
        let state = SortState::new(SortColumn::Type, SortDirection::Ascending);
        sort_items(&mut items, &state);
        assert_eq!(items[0].file_type, "md");
        assert_eq!(items[1].file_type, "pdf");
        assert_eq!(items[2].file_type, "rs");
    }
}
