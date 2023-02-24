//! Utility methods

#![warn(missing_docs)]

use std::collections::HashMap;
use std::ops::AddAssign;
use std::str::FromStr;

use bytesize::ByteSize;

/// Defined orderings for results
#[derive(Debug)]
pub enum SortMode {
    /// Sort by contained number of files
    Count,
    /// Sort by total size of internal files
    Total,
    /// Sort by average size of internal files
    Average,
    /// Sort by maximum size of internal files
    Max,
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
}

impl ChildSizeEntry {
    fn new() -> ChildSizeEntry {
        Default::default()
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
    }
}

impl FromStr for SortMode {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "count" => Ok(Self::Count),
            "total" => Ok(Self::Total),
            "average" => Ok(Self::Average),
            "max" => Ok(Self::Max),
            _ => Err("no match"),
        }
    }
}

/// Walk a path, recording details of all immediate children
pub fn walktree(path: &str) -> HashMap<String, ChildSizeEntry> {
    walkdir::WalkDir::new(path)
        .same_file_system(true)
        .into_iter()
        .flatten()
        .fold(HashMap::new(), |acc, e| filefold(acc, e, path))
}

/// Produce table to stdout, based on supplied sorting and direction
pub fn process(sort: SortMode, reverse: bool, entries: HashMap<String, ChildSizeEntry>) {
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
}

fn filefold(
    mut acc: HashMap<String, ChildSizeEntry>,
    e: walkdir::DirEntry,
    _base: &str,
) -> HashMap<String, ChildSizeEntry> {
    if e.file_type().is_file() {
        let key = e
            .path()
            .parent()
            .map(|n| n.to_string_lossy())
            .map(|n| n.to_string())
            .unwrap_or_else(|| "".to_string());
        if let Ok(metadata) = e.metadata() {
            let entry = acc.entry(key).or_insert_with(ChildSizeEntry::new);
            let size = ByteSize::b(metadata.len());
            *entry += size;
        }
    }
    acc
}
