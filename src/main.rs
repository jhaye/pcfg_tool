pub mod grammar;
pub mod parser;
pub mod tree;

use std::fs::File;
use std::io::{self, BufRead};
use std::str::FromStr;

use grammar::{GrammarAbsoluteWeight, GrammarNormalisedWeight};
use parser::SExp;
use tree::Tree;

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

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Induce { grammar } => {
            let stdin = io::stdin();
            let handle = stdin.lock();

            let grammar_absolute = handle
                .lines()
                .filter_map(|l| l.ok())
                .map(|l| SExp::from_str(&l))
                .filter_map(|s| s.ok())
                .map(|s| Tree::from(&s))
                .map(GrammarAbsoluteWeight::from)
                .fold(GrammarAbsoluteWeight::default(), |acc, x| acc.merge(x));

            let grammar_normalised = GrammarNormalisedWeight::from(grammar_absolute);

            // Write to files if grammar name was chosen, otherwise print to STDOUT.
            if let Some(grammar_name) = grammar {
                let mut rules_file = File::create(format!("{}.rules", grammar_name))?;
                grammar_normalised.write_non_lexical_rules(&mut rules_file)?;
                let mut lexicon_file = File::create(format!("{}.lexicon", grammar_name))?;
                grammar_normalised.write_lexical_rules(&mut lexicon_file)?;
                let mut words_file = File::create(format!("{}.words", grammar_name))?;
                grammar_normalised.write_terminals(&mut words_file)?;
            } else {
                let stdout = io::stdout();
                let mut out_handle = stdout.lock();

                grammar_normalised.write_non_lexical_rules(&mut out_handle)?;
                grammar_normalised.write_lexical_rules(&mut out_handle)?;
                grammar_normalised.write_terminals(&mut out_handle)?;
            }
        }
    }

    Ok(())
}
