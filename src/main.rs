pub mod grammar;
pub mod sentence;
pub mod sexp;
pub mod tree;

use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::PathBuf;
use std::str::FromStr;

use grammar::bare::GrammarBare;
use grammar::intified::GrammarIntified;
use grammar::rule::{Rule, WeightedRule};
use sentence::Sentence;
use sexp::SExp;
use tree::Tree;

use clap::{ArgEnum, Parser, Subcommand};

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
    Induce {
        grammar: Option<String>,
    },
    Parse {
        rules: String,
        lexicon: String,
        #[clap(short, long, default_value_t=ParsingParadigma::Cyk, arg_enum)]
        paradigma: ParsingParadigma,
        #[clap(short, long, default_value_t = String::from("ROOT"))]
        initial_nonterminal: String,
        #[clap(short, long)]
        unking: bool,
        #[clap(short, long)]
        smoothing: bool,
        #[clap(short, long)]
        threshold_beam: Option<u32>,
        #[clap(short, long)]
        rank_beam: Option<u32>,
        #[clap(short, long)]
        kbest: Option<u32>,
        #[clap(short, long)]
        astar: Option<PathBuf>,
    },
    Binarise {
        #[clap(short, long, default_value_t = 999)]
        horizontal: u32,
        #[clap(short, long, default_value_t = 1)]
        vertical: u32,
        #[clap(long)]
        help: bool,
    },
    Debinarise,
    Unk {
        #[clap(short, long)]
        threshold: Option<u32>,
    },
    Smooth {
        #[clap(short, long)]
        threshold: Option<u32>,
    },
    Outside {
        rules: String,
        lexicon: String,
        grammar: Option<String>,
        #[clap(short, long, default_value_t = String::from("ROOT"))]
        initial_nonterminal: String,
    },
}

#[derive(ArgEnum, Copy, Clone, PartialEq, Eq)]
enum ParsingParadigma {
    Cyk,
    Deductive,
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Induce { grammar } => {
            let stdin = io::stdin();
            let handle = stdin.lock();

            let grammar_absolute = handle
                .lines()
                .filter_map(|l| {
                    if l.is_err() {
                        eprintln!("Error when reading line: {:?}", l);
                    }
                    l.ok()
                })
                .map(|l| SExp::from_str(&l))
                .filter_map(|s| {
                    if s.is_err() {
                        eprintln!("Error when parsing SExp: {:?}", s);
                    }
                    s.ok()
                })
                .map(Tree::from)
                .map(GrammarBare::from)
                .fold(GrammarBare::default(), |acc, x| acc.merge(x));

            let grammar_normalised: GrammarBare<_, _, f64> = GrammarBare::from(grammar_absolute);

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
        Commands::Parse {
            rules,
            lexicon,
            paradigma,
            initial_nonterminal,
            unking,
            smoothing,
            threshold_beam,
            rank_beam,
            kbest,
            astar,
        } => {
            // Filter out all unsupported options
            if *unking
                || *smoothing
                || threshold_beam.is_some()
                || rank_beam.is_some()
                || kbest.is_some()
                || astar.is_some()
                || *paradigma == ParsingParadigma::Deductive
            {
                std::process::exit(22)
            }

            let mut grammar = GrammarIntified::new(initial_nonterminal.as_str().into());

            let rules_file = File::open(rules)?;
            let lexicon_file = File::open(lexicon)?;
            let rules_reader = BufReader::new(rules_file);
            let lexicon_reader = BufReader::new(lexicon_file);

            rules_reader
                .lines()
                .filter_map(|l| {
                    if l.is_err() {
                        eprintln!("Error when reading line: {:?}", l);
                    }
                    l.ok()
                })
                .map(|l| WeightedRule::from_str(&l))
                .filter_map(|r| {
                    if r.is_err() {
                        eprintln!("Error when parsing non-lexical rule: {:?}", r);
                    }

                    if let Ok(WeightedRule {
                        rule: Rule::Lexical { lhs: _, rhs: _ },
                        weight: _,
                    }) = r
                    {
                        eprintln!(
                            "Lexical rule parsed when parsing non-lexical rules: {:?}",
                            r
                        );
                        None
                    } else {
                        r.ok()
                    }
                })
                .for_each(|r| grammar.insert_rule(r));

            lexicon_reader
                .lines()
                .filter_map(|l| {
                    if l.is_err() {
                        eprintln!("Error when reading line: {:?}", l);
                    }
                    l.ok()
                })
                .map(|l| WeightedRule::from_str(&l))
                .filter_map(|r| {
                    if r.is_err() {
                        eprintln!("Error when parsing non-lexical rule: {:?}", r);
                    }

                    if let Ok(WeightedRule {
                        rule: Rule::NonLexical { lhs: _, rhs: _ },
                        weight: _,
                    }) = r
                    {
                        eprintln!(
                            "Non-lexical rule parsed when parsing lexical rules: {:?}",
                            r
                        );
                        None
                    } else {
                        r.ok()
                    }
                })
                .for_each(|r| grammar.insert_rule(r));

            let stdin = io::stdin();
            let handle = stdin.lock();
            let sentences: Vec<_> = handle
                .lines()
                .filter_map(|l| {
                    if l.is_err() {
                        eprintln!("Error when reading line: {:?}", l);
                    }
                    l.ok()
                })
                .map(|l| Sentence::from_str(&l))
                .filter_map(|s| {
                    if s.is_err() {
                        eprintln!("Error when parsing sentence: {:?}", s);
                    }
                    s.ok()
                })
                .collect();
        }
        _ => std::process::exit(22),
    }

    Ok(())
}
