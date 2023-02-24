use std::collections::HashMap;

use clap::Parser;

use childsize::{process, walktree, ChildSizeEntry};

#[derive(Parser, Debug)]
#[clap(version = "1.0", author = "Havvoric <havvoric@gmail.com>")]
struct Opts {
    paths: Vec<String>,
    #[clap(short, long, possible_values=["average", "count", "max", "total"])]
    sort: childsize::SortMode,
    #[clap(short, long)]
    reverse: bool,
}

fn main() {
    let opts: Opts = Opts::parse();
    println!("{opts:?}");

let mut entries: HashMap<String, ChildSizeEntry> = HashMap::new();

for path in opts.paths {
for (k, v) in walktree(&path) {
    entries.insert(k, v);
}
}
    process(opts.sort, opts.reverse, entries);
}
