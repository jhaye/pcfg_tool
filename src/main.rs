pub mod grammar;
pub mod parser;
pub mod tree;

use std::io::{self, BufRead};
use std::str::FromStr;

use parser::SExp;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
#[clap(propagate_version = true)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Reads a sequence of constituent trees from STDIN and prints the induced PCFG to STDOUT.
    /// If the optional argument [GRAMMAR] is present, it is written into the files
    /// GRAMMAR.rules, GRAMMAR.lexicon and GRAMMAR.words.
    Induce { grammar: Option<String> },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Induce { grammar } => {
            let stdin = io::stdin();
            let handle = stdin.lock();

            let sexps: Vec<SExp<String>> = handle
                .lines()
                .filter_map(|l| l.ok())
                .map(|l| SExp::from_str(&l))
                .filter_map(|s| s.ok())
                .collect();

            for sexp in sexps {
                println!("{:#?}", sexp);
            }
        }
    }
}
