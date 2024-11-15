//! Utility methods

use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::Display;
use std::ops::AddAssign;
use std::str::FromStr;

use bytesize::ByteSize;
use clap::Parser;

#[derive(Parser, Debug)]
#[clap(version = "1.0", author = "Havvoric <havvoric@gmail.com>")]
pub struct Opts {
    pub paths: Vec<String>,
    #[clap(short, long = "pattern")]
    pub patterns: Vec<String>,

    #[clap(short, long, value_enum)]
    sort: SortMode,
    #[clap(short, long)]
    reverse: bool,
    #[clap(short = 'z', long)]
    show_summary: bool,
}

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
#[derive(Debug)]
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
    fn update_average(&mut self) {
        self.average = ByteSize::b((self.total.as_u64() as f64 / self.count as f64) as u64);
    }
}

impl Default for ChildSizeEntry {
    fn default() -> Self {
        Self {
            count: Default::default(),
            total: Default::default(),
            average: Default::default(),
            max: Default::default(),
            min: ByteSize::pib(1),
        }
    }
}

impl Display for ChildSizeEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} {} {}",
            self.count, self.total, self.average, self.max
        )
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

type OrderProc = fn(&(&String, &ChildSizeEntry), &(&String, &ChildSizeEntry)) -> Ordering;

impl SortMode {
    fn ordering(&self) -> OrderProc {
        match self {
            SortMode::Count => |a, b| a.1.count.partial_cmp(&b.1.count).unwrap(),
            SortMode::Average => |a, b| a.1.average.partial_cmp(&b.1.average).unwrap(),
            SortMode::Total => |a, b| a.1.total.partial_cmp(&b.1.total).unwrap(),
            SortMode::Max => |a, b| a.1.max.partial_cmp(&b.1.max).unwrap(),
            SortMode::Min => |a, b| a.1.min.partial_cmp(&b.1.min).unwrap(),
        }
    }
}

#[derive(Debug)]
pub struct Processor {
    summary: ChildSizeEntry,
    entries: HashMap<String, ChildSizeEntry>,
    opts: Opts,
}

impl Processor {
    pub fn new(opts: Opts) -> Self {
        Self {
            opts,
            summary: ChildSizeEntry {
                min: ByteSize::pib(1),
                count: 0,
                total: bytesize::ByteSize(0),
                average: bytesize::ByteSize(0),
                max: bytesize::ByteSize(0),
            },
            entries: HashMap::new(),
        }
    }

    /// Walk a path, recording details of all immediate children
    fn walktree(&mut self, path: &str, globset: &globset::GlobSet) {
        for entry in walkdir::WalkDir::new(path)
            .same_file_system(true)
            .into_iter()
            .flatten()
        {
            self.filefold(entry, path, globset);
        }
        //.fold(self, |acc, e| self.filefold(acc, e, path, globset));
    }

    /// Walk all paths, recording details of encountered files
    pub fn walktrees(&mut self) {
        let mut builder = globset::GlobSetBuilder::new();
        for glob in &self.opts.patterns {
            builder.add(globset::Glob::new(glob).unwrap());
        }
        let globset = builder.build().unwrap();

        let paths = self.opts.paths.clone();
        for path in paths {
            self.walktree(&path, &globset);
        }
    }

    /// Produce table to stdout, based on supplied sorting and direction
    pub fn process(&mut self) {
        let mut entries: Vec<(&String, &ChildSizeEntry)> = Vec::new();
        for (file, entry) in self.entries.iter_mut() {
            entry.update_average();
            entries.push((file, entry));
        }
        entries.sort_unstable_by(self.opts.sort.ordering());
        if self.opts.reverse {
            entries.reverse();
        }
        for entry in entries {
            println!("{} {}", entry.1, entry.0);
        }
        if self.opts.show_summary {
            self.summary.update_average();
            let summary = format!("{} SUMMARY", self.summary);
            println!("{}", "=".repeat(summary.len()));
            println!("{}", summary);
            println!("{}", "=".repeat(summary.len()));
        }
    }

    fn filefold(&mut self, e: walkdir::DirEntry, base: &str, globset: &globset::GlobSet) {
        if e.file_type().is_file() && globset.is_match(e.file_name()) {
            let key = Self::key(e.path(), base).unwrap_or_default();
            if let Ok(metadata) = e.metadata() {
                // println!("key={}, path={:?}, base={}, file_name={:?}", key, e.path(), base, e.file_name());
                let entry = self.entries.entry(key).or_default();
                let size = ByteSize::b(metadata.len());
                *entry += size;
                self.summary += size;
            }
        }
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
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_key_long() {
        let path = Path::new("/test/1/2/3/4/5.txt");
        let r = Processor::key(path, "/test/");
        assert!(r.is_some());
        assert_eq!(r.unwrap(), "/test/1");
    }

    #[test]
    fn test_key_normal() {
        let path = Path::new("/test/1/5.txt");
        let r = Processor::key(path, "/test/");
        assert!(r.is_some());
        assert_eq!(r.unwrap(), "/test/1");
    }

    #[test]
    fn test_key_no_subdir() {
        let path = Path::new("/test/5.txt");
        let r = Processor::key(path, "/test/");
        assert!(r.is_some());
        assert_eq!(r.unwrap(), "/test/");
    }
    #[test]
    fn test_key_not_rooted() {
        let path = Path::new("/test/1/5.txt");
        let r = Processor::key(path, "/test2/");
        assert!(r.is_none());
    }
}
