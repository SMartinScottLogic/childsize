use std::collections::HashMap;

use clap::Parser;

use childsize::{process, walktree, ChildSizeEntry};

#[derive(Parser, Debug)]
#[clap(version = "1.0", author = "Havvoric <havvoric@gmail.com>")]
struct Opts {
    paths: Vec<String>,
    #[clap(short, long, value_enum)]
    sort: childsize::SortMode,
    #[clap(short, long)]
    reverse: bool,
    #[clap(short, long = "pattern")]
    patterns: Vec<String>,
    #[clap(short = 'z', long)]
    show_summary: bool,
}

fn main() {
    let opts: Opts = Opts::parse();
    println!("{opts:?}");

    let mut entries: HashMap<String, ChildSizeEntry> = HashMap::new();

    let mut builder = globset::GlobSetBuilder::new();
    for glob in opts.patterns {
        builder.add(globset::Glob::new(&glob).unwrap());
    }
    let globset = builder.build().unwrap();

    for path in opts.paths {
        for (k, v) in walktree(&path, &globset) {
            entries.insert(k, v);
        }
    }
    process(opts.sort, opts.reverse, opts.show_summary, entries);
}
