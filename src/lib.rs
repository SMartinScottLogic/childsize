//! Utility methods

#![warn(missing_docs)]

use std::collections::HashMap;
use std::ops::AddAssign;
use std::str::FromStr;

use bytesize::ByteSize;

/// Defined orderings for results
#[derive(Debug, Clone)]
pub enum SortMode {
    /// Sort by contained number of files
    Count,
    /// Sort by total size of internal files
    Total,
    /// Sort by average size of internal files
    Average,
    /// Sort by maximum size of internal files
    Max,
    /// Sort by maximum size of internal files
    Min,
}

/// Details of internal files within a folder
#[derive(Default)]
pub struct ChildSizeEntry {
    /// Number of internal files
    count: u64,
    /// Total size of internal files
    total: ByteSize,
    /// Average size of internal files
    average: ByteSize,
    /// Maximum size of internal files
    max: ByteSize,
    /// Minimum size of internal files
    min: ByteSize,
}

impl ChildSizeEntry {
    fn new() -> ChildSizeEntry {
        Self {
            min: ByteSize::pib(1),
            ..Default::default()
        }
    }
    fn update_average(&mut self) {
        self.average = ByteSize::b((self.total.as_u64() as f64 / self.count as f64) as u64);
    }
}

impl AddAssign<ByteSize> for ChildSizeEntry {
    fn add_assign(&mut self, size: ByteSize) {
        self.count += 1;
        self.total += size;
        if self.max < size {
            self.max = size;
        }
        if self.min > size {
            self.min = size;
        }
    }
}

static mut SUMMARY: ChildSizeEntry = ChildSizeEntry {
    min: ByteSize::pib(1),
    count: 0,
    total: bytesize::ByteSize(0),
    average: bytesize::ByteSize(0),
    max: bytesize::ByteSize(0),
};

impl FromStr for SortMode {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "count" => Ok(Self::Count),
            "total" => Ok(Self::Total),
            "average" => Ok(Self::Average),
            "max" => Ok(Self::Max),
            "min" => Ok(Self::Min),
            _ => Err("no match"),
        }
    }
}

/// Walk a path, recording details of all immediate children
pub fn walktree(path: &str, globset: &globset::GlobSet) -> HashMap<String, ChildSizeEntry> {
    walkdir::WalkDir::new(path)
        .same_file_system(true)
        .into_iter()
        .flatten()
        .fold(HashMap::new(), |acc, e| filefold(acc, e, path, globset))
}

/// Produce table to stdout, based on supplied sorting and direction
pub fn process(
    sort: SortMode,
    reverse: bool,
    show_summary: bool,
    entries: HashMap<String, ChildSizeEntry>,
) {
    let mut entries: Vec<_> = entries
        .into_iter()
        .map(|(file, mut entry)| {
            entry.update_average();
            (file, entry)
        })
        .collect();
    match sort {
        SortMode::Count => {
            entries.sort_unstable_by(|a, b| a.1.count.partial_cmp(&b.1.count).unwrap())
        }
        SortMode::Average => {
            entries.sort_unstable_by(|a, b| a.1.average.partial_cmp(&b.1.average).unwrap())
        }
        SortMode::Total => {
            entries.sort_unstable_by(|a, b| a.1.total.partial_cmp(&b.1.total).unwrap())
        }
        SortMode::Max => entries.sort_unstable_by(|a, b| a.1.max.partial_cmp(&b.1.max).unwrap()),
        SortMode::Min => entries.sort_unstable_by(|a, b| a.1.min.partial_cmp(&b.1.min).unwrap()),
    }
    if reverse {
        entries.reverse();
    }
    for entry in entries {
        println!(
            "{} {} {} {} {}",
            entry.1.count, entry.1.total, entry.1.average, entry.1.max, entry.0
        );
    }
    if show_summary {
        let summary = unsafe {
            SUMMARY.update_average();
            format!(
                "{} {} {} {} {}",
                SUMMARY.count, SUMMARY.total, SUMMARY.average, SUMMARY.max, "SUMMARY"
            )
        };
        println!("{}", "=".repeat(summary.len()));
        println!("{}", summary);
        println!("{}", "=".repeat(summary.len()));
    }
}

fn filefold(
    mut acc: HashMap<String, ChildSizeEntry>,
    e: walkdir::DirEntry,
    base: &str,
    globset: &globset::GlobSet,
) -> HashMap<String, ChildSizeEntry> {
    if e.file_type().is_file() && globset.is_match(e.file_name()) {
        let key = key(e.path(), base).unwrap_or_default();
        if let Ok(metadata) = e.metadata() {
            //println!("key={}, path={:?}, base={}, file_name={:?}", key, e.path(), base, e.file_name());
            let entry = acc.entry(key).or_insert_with(ChildSizeEntry::new);
            let size = ByteSize::b(metadata.len());
            *entry += size;
            unsafe { SUMMARY += size };
        }
    }
    acc
}

fn key(path: &std::path::Path, base: &str) -> Option<String> {
    let r = match path.strip_prefix(base) {
        Ok(r) => r,
        Err(_) =>
        // should be inside base
        {
            return None
        }
    };
    match r.parent() {
        None => Some(base.to_string()),
        Some(p) if p == std::path::Path::new("") => Some(base.to_string()),
        Some(parent) => Some(
            std::path::Path::new(base)
                .join(parent)
                .as_os_str()
                .to_string_lossy()
                .to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_key_long() {
        let path = Path::new("/test/1/2/3/4/5.txt");
        let r = key(path, "/test/");
        assert!(r.is_some());
        assert_eq!(r.unwrap(), "/test/1");
    }

    #[test]
    fn test_key_normal() {
        let path = Path::new("/test/1/5.txt");
        let r = key(path, "/test/");
        assert!(r.is_some());
        assert_eq!(r.unwrap(), "/test/1");
    }

    #[test]
    fn test_key_no_subdir() {
        let path = Path::new("/test/5.txt");
        let r = key(path, "/test/");
        assert!(r.is_some());
        assert_eq!(r.unwrap(), "/test/");
    }
    #[test]
    fn test_key_not_rooted() {
        let path = Path::new("/test/1/5.txt");
        let r = key(path, "/test2/");
        assert!(r.is_none());
    }
}
