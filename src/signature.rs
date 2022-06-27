use std::fmt;

pub enum LetterSuffix {
    AllCapitalised,
    StartCapitalised,
    Capitalised,
    HasLower,
    HasLetter,
    NoLetter,
}

pub enum NumberSuffix {
    FullNumber,
    ContainsNumber,
}

pub struct UnkSignature {
    letter_suffix: Option<LetterSuffix>,
    number_suffix: Option<NumberSuffix>,
    has_dash: bool,
    has_period: bool,
    has_comma: bool,
    word_suffix: Option<String>,
}

impl UnkSignature {
    pub fn new(word: &str, idx: usize) -> Self {
        if word.is_empty() {
            UnkSignature {
                letter_suffix: None,
                number_suffix: None,
                has_dash: false,
                has_period: false,
                has_comma: false,
                word_suffix: None,
            }
        } else {
            let mut has_lower = false;
            let mut has_alphabetic = false;
            let mut has_numeric = false;
            let mut has_nonnumeric = false;

            for c in word.chars() {
                if c.is_lowercase() {
                    has_lower = true;
                } else if c.is_alphabetic() {
                    has_alphabetic = true;
                }

                if c.is_numeric() {
                    has_numeric = true;
                } else {
                    has_nonnumeric = true;
                }
            }

            let word_suffix = if word.len() > 3 {
                let last_char = word.chars().last().unwrap();

                if last_char.is_alphabetic() {
                    Some(last_char.to_lowercase().to_string())
                } else {
                    None
                }
            } else {
                None
            };

            let letter_suffix = if word.chars().next().unwrap().is_uppercase() {
                if !has_lower {
                    LetterSuffix::AllCapitalised
                } else if idx == 0 {
                    LetterSuffix::StartCapitalised
                } else {
                    LetterSuffix::Capitalised
                }
            } else if has_lower {
                LetterSuffix::HasLower
            } else if has_alphabetic {
                LetterSuffix::HasLetter
            } else {
                LetterSuffix::NoLetter
            };
            let number_suffix = if has_numeric && !has_nonnumeric {
                Some(NumberSuffix::FullNumber)
            } else if has_numeric {
                Some(NumberSuffix::ContainsNumber)
            } else {
                None
            };

            UnkSignature {
                letter_suffix: Some(letter_suffix),
                number_suffix,
                has_dash: word.contains('-'),
                has_period: word.contains('.'),
                has_comma: word.contains(','),
                word_suffix,
            }
        }
    }
}

impl fmt::Display for UnkSignature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut result = write!(f, "UNK");

        if let Some(letter_suffix) = self.letter_suffix.as_ref() {
            let suffix = match letter_suffix {
                LetterSuffix::AllCapitalised => "-AC",
                LetterSuffix::StartCapitalised => "-SC",
                LetterSuffix::Capitalised => "-C",
                LetterSuffix::HasLower => "-L",
                LetterSuffix::HasLetter => "-U",
                LetterSuffix::NoLetter => "-S",
            };

            result = result.and(write!(f, "{}", suffix));
        }

        if let Some(number_suffix) = self.number_suffix.as_ref() {
            let suffix = match number_suffix {
                NumberSuffix::FullNumber => "-N",
                NumberSuffix::ContainsNumber => "-n",
            };

            result = result.and(write!(f, "{}", suffix));
        }

        if self.has_dash {
            result = result.and(write!(f, "-H"));
        }

        if self.has_period {
            result = result.and(write!(f, "-P"));
        }

        if self.has_comma {
            result = result.and(write!(f, "-C"));
        }

        if let Some(ref word_suffix) = self.word_suffix {
            result = result.and(write!(f, "-{}", word_suffix));
        }

        result
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_signature() {
        assert_eq!("UNK".to_string(), format!("{}", UnkSignature::new("", 0)));

        assert_eq!(
            "UNK-AC".to_string(),
            format!("{}", UnkSignature::new("USA", 0))
        );

        assert_eq!(
            "UNK-SC".to_string(),
            format!("{}", UnkSignature::new("Me", 0))
        );

        assert_eq!(
            "UNK-C".to_string(),
            format!("{}", UnkSignature::new("Me", 1))
        );

        assert_eq!(
            "UNK-L".to_string(),
            format!("{}", UnkSignature::new("me", 1))
        );

        assert_eq!(
            "UNK-L".to_string(),
            format!("{}", UnkSignature::new("me", 1))
        );

        assert_eq!(
            "UNK-U-n".to_string(),
            format!("{}", UnkSignature::new("2U", 1))
        );

        assert_eq!(
            "UNK-S-N".to_string(),
            format!("{}", UnkSignature::new("7", 1))
        );

        assert_eq!(
            "UNK-AC-H".to_string(),
            format!("{}", UnkSignature::new("A-Z", 1))
        );

        assert_eq!(
            "UNK-C-P".to_string(),
            format!("{}", UnkSignature::new("Dr.", 1))
        );

        assert_eq!(
            "UNK-L-C".to_string(),
            format!("{}", UnkSignature::new("or,", 1))
        );

        assert_eq!(
            "UNK-L-d".to_string(),
            format!("{}", UnkSignature::new("word", 1))
        );

        assert_eq!(
            "UNK-L-n".to_string(),
            format!("{}", UnkSignature::new("cloud9", 1))
        );
    }
}
