use clap::Parser;

use childsize::{Opts, Processor};

fn main() {
    let opts: Opts = Opts::parse();
    println!("{opts:?}");

    let mut processor = Processor::new(opts);
    processor.walktrees();
    //println!("processor: {:?}", processor);
    processor.process();
}
