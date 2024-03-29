pub mod binarized;
pub mod grammar;
pub mod sentence;
pub mod sexp;
pub mod signature;
pub mod tree;
pub mod unk;

use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::PathBuf;
use std::str::FromStr;

use clap::{ArgEnum, Parser, Subcommand};
use fxhash::FxHashMap;
use rayon::prelude::*;

use grammar::bare::GrammarBare;
use grammar::parse::{GrammarParse, PruneMode};
use grammar::rule::{Rule, WeightedRule};
use sentence::Sentence;
use sexp::SExp;
use tree::Tree;

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
    /// Reads a sequence of sentences from STDIN and returns the best derived parse trees to STDOUT.
    /// RULES and LEXICON are the files that make up the used PCFG.
    Parse {
        rules: String,
        lexicon: String,
        /// Choose the parsing paradigm.
        #[clap(short, long, default_value_t=ParsingParadigma::Cyk, arg_enum)]
        paradigma: ParsingParadigma,
        /// Set custom initial non-terminal for the given PCFG.        /// Not implemented.

        #[clap(short, long, default_value_t = String::from("ROOT"))]
        initial_nonterminal: String,
        /// Do trivial unking on supplied sentences before parsing.
        #[clap(short, long)]
        unking: bool,
        /// Do smoothing on supplied sentences before parsing.
        #[clap(short, long)]
        smoothing: bool,
        /// Prune parsing data with the given threshold. Rules are only kept if their probability
        /// is not lower than the best derivation multiplied by the threshold.
        #[clap(short, long)]
        threshold_beam: Option<f64>,
        /// Prune parsing data with the given rank n. Rules are only kept if their probability
        /// is not lower than the n-best derivation.
        #[clap(short, long)]
        rank_beam: Option<usize>,
        /// Not implemented.
        #[clap(short, long)]
        kbest: Option<u32>,
        /// Not implemented.
        #[clap(short, long)]
        astar: Option<PathBuf>,
    },
    /// Reads constituent trees from STDIN and returns their binarised counterparts to STDOUT.
    Binarise {
        /// Set horizontal markovisation parameter.
        #[clap(short, long, default_value_t = 999)]
        horizontal: usize,
        /// Set vertical markovisation parameter.
        #[clap(short, long, default_value_t = 1)]
        vertical: usize,
        #[clap(long)]
        help: bool,
    },
    /// Reads binarised constituent trees from STDIN and returns them in their original state to STDOUT.
    Debinarise,
    /// Reads sequence of constituent trees from STDIN and returns the derived trees via trivial unking.
    Unk {
        /// If a word occurs less often than the threshold it gets unked.
        #[clap(short, long)]
        threshold: usize,
    },
    /// Reads sequence of constituent trees from STDIN and returns the derived trees via smoothing.
    Smooth {
        /// If a word occurs less often than the threshold it gets unked with the derived signature.
        #[clap(short, long)]
        threshold: usize,
    },
    /// Not implemented.
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
            if kbest.is_some() || astar.is_some() || *paradigma == ParsingParadigma::Deductive {
                std::process::exit(22)
            }

            if *unking && *smoothing {
                panic!("Unking and smoothing are mutually exclusive. Only use one!")
            }

            let mode = PruneMode {
                threshold: *threshold_beam,
                fixed_size: *rank_beam,
            };

            let mut grammar = GrammarParse::new(initial_nonterminal.as_str().into());

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
                        eprintln!("Error when parsing lexical rule: {:?}", r);
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

            const LINES_READ: usize = 128;
            let stdin = io::stdin();
            let mut handle = stdin.lock();
            let mut input_buf = String::new();
            let mut done = false;
            while !done {
                for _ in 0..LINES_READ {
                    match handle.read_line(&mut input_buf) {
                        Ok(0) => {
                            done = true;
                            break;
                        }
                        Ok(_) => {}
                        Err(x) => eprintln!("Error when reading line: {:?}", x),
                    }
                }

                let trees: Vec<_> = if *unking || *smoothing {
                    input_buf
                        .par_lines()
                        .map(Sentence::from_str)
                        .filter_map(|s| {
                            if s.is_err() {
                                eprintln!("Error when parsing sentence: {:?}", s);
                            }
                            s.ok()
                        })
                        .map(|mut s| {
                            // Unking and smoothing are effectively the same operation, but
                            // smoothing is more fine grained.
                            let wmap = if *unking {
                                s.unkify(&grammar.rules_lexical)
                            } else {
                                // smoothing
                                s.smooth(&grammar.rules_lexical)
                            };
                            (s, wmap)
                        })
                        .map(|(s, wmap)| {
                            (
                                grammar.cyk(&s, &mode).unwrap_or_else(|| s.into_noparse()),
                                wmap,
                            )
                        })
                        .map(|(mut t, wmap)| {
                            if let Some(wmap) = wmap {
                                t.deunkify(wmap);
                            }
                            t
                        })
                        .collect()
                } else {
                    input_buf
                        .par_lines()
                        .map(Sentence::from_str)
                        .filter_map(|s| {
                            if s.is_err() {
                                eprintln!("Error when parsing sentence: {:?}", s);
                            }
                            s.ok()
                        })
                        .map(|s| grammar.cyk(&s, &mode).unwrap_or_else(|| s.into_noparse()))
                        .collect()
                };

                for tree in trees {
                    println!("{}", tree);
                }

                input_buf.clear();
            }
        }
        Commands::Binarise {
            horizontal,
            vertical,
            ..
        } => {
            let stdin = io::stdin();
            let handle = stdin.lock();

            handle
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
                .map(|t| t.markovize(*vertical, *horizontal, &[]))
                .for_each(|t| println!("{}", t));
        }
        Commands::Debinarise => {
            let stdin = io::stdin();
            let handle = stdin.lock();

            handle
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
                .map(Tree::parse_markovized)
                .map(Tree::debinarize)
                .for_each(|t| println!("{}", t));
        }
        Commands::Unk { threshold } => {
            unking(UnkingMode::Trivial, *threshold);
        }
        Commands::Smooth { threshold } => {
            unking(UnkingMode::Smoothing, *threshold);
        }
        _ => std::process::exit(22),
    }

    Ok(())
}

enum UnkingMode {
    Trivial,
    Smoothing,
}

fn unking(mode: UnkingMode, threshold: usize) {
    let stdin = io::stdin();
    let handle = stdin.lock();

    let mut word_count = FxHashMap::default();

    let mut trees: Vec<_> = handle
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
        .collect();

    for tree in &trees {
        unk::count_words(tree, &mut word_count);
    }

    // We keep all words that we don't want to unkify.
    word_count.retain(|_, v| *v > threshold);
    let word_count = word_count;

    trees.iter_mut().for_each(|t| {
        match mode {
            UnkingMode::Trivial => t.unkify(&word_count),
            UnkingMode::Smoothing => t.smooth(&word_count),
        };
        println!("{}", t);
    });
}
