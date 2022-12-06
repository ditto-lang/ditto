use crate::{Comment, Span};
use logos::{Logos, SpannedIter};

pub struct Lexer<'input> {
    pub comments: Vec<Comment>,
    raw_token_stream: std::iter::Peekable<SpannedIter<'input, RawToken>>,
}

impl<'input> Lexer<'input> {
    pub fn new(input: &'input str) -> Self {
        Self {
            comments: Vec::new(),
            raw_token_stream: RawToken::lexer(input).spanned().peekable(),
        }
    }

    fn collect_comments(&mut self) -> Comments {
        let leading = std::mem::take(&mut self.comments);
        let has_trailing_comment = matches!(
            self.raw_token_stream.peek(),
            Some((RawToken::Comment(_), _))
        );
        if has_trailing_comment {
            if let Some((RawToken::Comment(string), _)) = self.raw_token_stream.next() {
                return Comments {
                    leading,
                    trailing: Some(Comment(string)),
                };
            } else {
                unreachable!()
            }
        }
        Comments {
            leading,
            trailing: None,
        }
    }

    // TODO: pub method for draining any dangling comments?
}

#[derive(Debug)]
pub enum Error {
    InvalidToken(Span),
}

impl<'input> Iterator for Lexer<'input> {
    type Item = Result<(usize, Token, usize), Error>;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.raw_token_stream.next();
        let (start_offset, raw_token, end_offset) = match next {
            None => return None,
            Some((RawToken::Error, range)) => {
                return Some(Err(Error::InvalidToken(Span {
                    start_offset: range.start,
                    end_offset: range.end,
                })))
            }
            Some((RawToken::Newline, _)) => {
                // skip
                return self.next();
            }
            Some((RawToken::Comment(string), _)) => {
                self.comments.push(Comment(string));
                return self.next();
            }
            Some((raw_token, range)) => (range.start, raw_token, range.end),
        };

        let token = match raw_token {
            // Handled above ☝️
            RawToken::Comment(_) | RawToken::Newline | RawToken::Error => unreachable!(),

            RawToken::Dot => Token::Dot(self.collect_comments()),
            RawToken::DoubleDot => Token::DoubleDot(self.collect_comments()),
            RawToken::Comma => Token::Comma(self.collect_comments()),
            RawToken::Colon => Token::Colon(self.collect_comments()),
            RawToken::Semicolon => Token::Semicolon(self.collect_comments()),
            RawToken::Equals => Token::Equals(self.collect_comments()),
            RawToken::OpenParen => Token::OpenParen(self.collect_comments()),
            RawToken::CloseParen => Token::CloseParen(self.collect_comments()),
            RawToken::OpenBracket => Token::OpenBracket(self.collect_comments()),
            RawToken::CloseBracket => Token::CloseBracket(self.collect_comments()),
            RawToken::OpenBrace => Token::OpenBrace(self.collect_comments()),
            RawToken::CloseBrace => Token::CloseBrace(self.collect_comments()),
            RawToken::LeftArrow => Token::LeftArrow(self.collect_comments()),
            RawToken::RightArrow => Token::RightArrow(self.collect_comments()),
            RawToken::Pipe => Token::Pipe(self.collect_comments()),
            RawToken::ModuleKeyword => Token::ModuleKeyword(self.collect_comments()),
            RawToken::ExportsKeyword => Token::ExportsKeyword(self.collect_comments()),
            RawToken::ImportKeyword => Token::ImportKeyword(self.collect_comments()),
            RawToken::AsKeyword => Token::AsKeyword(self.collect_comments()),
            RawToken::TrueKeyword => Token::TrueKeyword(self.collect_comments()),
            RawToken::FalseKeyword => Token::FalseKeyword(self.collect_comments()),
            RawToken::UnitKeyword => Token::UnitKeyword(self.collect_comments()),
            RawToken::IfKeyword => Token::IfKeyword(self.collect_comments()),
            RawToken::ThenKeyword => Token::ThenKeyword(self.collect_comments()),
            RawToken::ElseKeyword => Token::ElseKeyword(self.collect_comments()),
            RawToken::TypeKeyword => Token::TypeKeyword(self.collect_comments()),
            RawToken::ForeignKeyword => Token::ForeignKeyword(self.collect_comments()),
            RawToken::MatchKeyword => Token::MatchKeyword(self.collect_comments()),
            RawToken::WithKeyword => Token::WithKeyword(self.collect_comments()),
            RawToken::LetKeyword => Token::LetKeyword(self.collect_comments()),
            RawToken::DoKeyword => Token::DoKeyword(self.collect_comments()),
            RawToken::ReturnKeyword => Token::ReturnKeyword(self.collect_comments()),
            RawToken::FnKeyword => Token::FnKeyword(self.collect_comments()),
            RawToken::EndKeyword => Token::EndKeyword(self.collect_comments()),
            RawToken::AliasKeyword => Token::AliasKeyword(self.collect_comments()),
            RawToken::RightPizzaOperator => Token::RightPizzaOperator(self.collect_comments()),
            RawToken::Name(string) => Token::Name((self.collect_comments(), string)),
            RawToken::ProperName(string) => Token::ProperName((self.collect_comments(), string)),
            RawToken::UnusedName(string) => Token::UnusedName((self.collect_comments(), string)),
            RawToken::PackageName(string) => Token::PackageName((self.collect_comments(), string)),
            RawToken::String(string) => Token::String((
                self.collect_comments(),
                // Remove the surrounding quotes
                string[1..string.len() - 1].to_owned(),
            )),
            RawToken::Number(string) => {
                if string.contains('.') {
                    Token::Float((self.collect_comments(), string))
                } else {
                    Token::Int((self.collect_comments(), string))
                }
            }
        };
        Some(Ok((start_offset, token, end_offset)))
    }
}

#[derive(Debug, Clone)]
pub enum Token {
    Dot(Comments),
    DoubleDot(Comments),
    Comma(Comments),
    Colon(Comments),
    Semicolon(Comments),
    Equals(Comments),
    OpenParen(Comments),
    CloseParen(Comments),
    OpenBracket(Comments),
    CloseBracket(Comments),
    OpenBrace(Comments),
    CloseBrace(Comments),
    LeftArrow(Comments),
    RightArrow(Comments),
    Pipe(Comments),
    ModuleKeyword(Comments),
    ExportsKeyword(Comments),
    ImportKeyword(Comments),
    AsKeyword(Comments),
    TrueKeyword(Comments),
    FalseKeyword(Comments),
    UnitKeyword(Comments),
    IfKeyword(Comments),
    ThenKeyword(Comments),
    ElseKeyword(Comments),
    TypeKeyword(Comments),
    ForeignKeyword(Comments),
    MatchKeyword(Comments),
    WithKeyword(Comments),
    LetKeyword(Comments),
    DoKeyword(Comments),
    ReturnKeyword(Comments),
    FnKeyword(Comments),
    EndKeyword(Comments),
    AliasKeyword(Comments),
    RightPizzaOperator(Comments),
    Name((Comments, String)),
    ProperName((Comments, String)),
    UnusedName((Comments, String)),
    PackageName((Comments, String)),
    String((Comments, String)),
    Int((Comments, String)),
    Float((Comments, String)),
}

#[derive(Debug, Clone)]
pub struct Comments {
    pub leading: Vec<Comment>,
    pub trailing: Option<Comment>,
}

#[derive(Logos, Debug, PartialEq)]
enum RawToken {
    #[token(".")]
    Dot,
    #[token("..")]
    DoubleDot,
    #[token(",")]
    Comma,
    #[token(":")]
    Colon,
    #[token(";")]
    Semicolon,
    #[token("=")]
    Equals,
    #[token("(")]
    OpenParen,
    #[token(")")]
    CloseParen,
    #[token("[")]
    OpenBracket,
    #[token("]")]
    CloseBracket,
    #[token("{")]
    OpenBrace,
    #[token("}")]
    CloseBrace,
    #[token("<-")]
    LeftArrow,
    #[token("->")]
    RightArrow,
    #[token("|")]
    Pipe,
    #[token("module")]
    ModuleKeyword,
    #[token("exports")]
    ExportsKeyword,
    #[token("import")]
    ImportKeyword,
    #[token("as")]
    AsKeyword,
    #[token("true")]
    TrueKeyword,
    #[token("false")]
    FalseKeyword,
    #[token("unit")]
    UnitKeyword,
    #[token("if")]
    IfKeyword,
    #[token("then")]
    ThenKeyword,
    #[token("else")]
    ElseKeyword,
    #[token("type")]
    TypeKeyword,
    #[token("foreign")]
    ForeignKeyword,
    #[token("match")]
    MatchKeyword,
    #[token("with")]
    WithKeyword,
    #[token("let")]
    LetKeyword,
    #[token("do")]
    DoKeyword,
    #[token("return")]
    ReturnKeyword,
    #[token("fn")]
    FnKeyword,
    #[token("end")]
    EndKeyword,
    #[token("alias")]
    AliasKeyword,

    #[token("|>")]
    RightPizzaOperator,

    #[regex(r"[a-z]\w*", priority = 2, callback = |lex| lex.slice().parse())]
    Name(String), //     ^^ Needs to be higher priority than PackageName

    #[regex(r"[A-Z]\w*", callback = |lex| lex.slice().parse())]
    ProperName(String),

    #[regex(r"_(?:[a-z]\w*)?", callback = |lex| lex.slice().parse())]
    UnusedName(String),

    #[regex(r"[a-z][a-z0-9-]*", callback = |lex| lex.slice().parse())]
    PackageName(String),

    #[regex(r"--[^\n]*", callback = |lex| lex.slice().parse())]
    Comment(String),

    #[regex(r"\d[\d_]*(?:\.\d[\d_]*)?", callback = |lex| lex.slice().parse())]
    Number(String),

    // TODO: improve this regex
    #[regex(r#""[^"]*""#, callback = |lex| lex.slice().parse())]
    String(String),

    #[regex(r"\r?\n")]
    Newline,

    // NOTE: this doesn't cover all non-newline whitespace,
    // but I'm not stressing that rn
    #[regex(r"[ \t]+", logos::skip)]
    #[error]
    Error,
}

#[cfg(test)]
mod tests {
    use super::{Comments, Error, Lexer, Token};

    #[test]
    fn it_lexes_as_expected() {
        let mut lexer = Lexer::new(
            r"  unit 
                match with
                -- comment 0
                do 
                --   comment 1
                -- comment 2
                return
                module     -- comment 3


                -- comment 4
                type -- comment 5
                -- comment 6

                -- comment 7
                exports -- comment 8
        ",
        );
        assert!(matches!(
            lexer.next(),
            Some(Ok((_, Token::UnitKeyword { .. }, _)))
        ));
        assert!(matches!(
            lexer.next(),
            Some(Ok((_, Token::MatchKeyword { .. }, _)))
        ));
        assert!(matches!(
            lexer.next(),
            Some(Ok((_, Token::WithKeyword { .. }, _)))
        ));
        if let Some(Ok((_, Token::DoKeyword(Comments { leading, trailing }), _))) = lexer.next() {
            assert_eq!(leading.len(), 1);
            assert_eq!(leading[0].0, "-- comment 0");
            assert!(trailing.is_none());
        } else {
            panic!()
        }
        if let Some(Ok((_, Token::ReturnKeyword(Comments { leading, trailing }), _))) = lexer.next()
        {
            assert_eq!(leading.len(), 2);
            assert!(trailing.is_none());
        } else {
            panic!()
        }
        if let Some(Ok((_, Token::ModuleKeyword(Comments { leading, trailing }), _))) = lexer.next()
        {
            assert!(leading.is_empty());
            assert!(trailing.is_some());
            assert_eq!(trailing.unwrap().0, "-- comment 3");
        } else {
            panic!()
        }
        if let Some(Ok((_, Token::TypeKeyword(Comments { leading, trailing }), _))) = lexer.next() {
            assert_eq!(leading.len(), 1);
            assert_eq!(leading[0].0, "-- comment 4");
            assert!(trailing.is_some());
        } else {
            panic!()
        }
        if let Some(Ok((_, Token::ExportsKeyword(Comments { leading, trailing }), _))) =
            lexer.next()
        {
            assert_eq!(leading.len(), 2);
            assert!(trailing.is_some());
            assert_eq!(trailing.unwrap().0, "-- comment 8");
        } else {
            panic!()
        }
    }

    macro_rules! assert_token {
        ($input:expr, $want:pat_param) => {{
            let mut lexer = crate::lexer::Lexer::new($input);
            let next = lexer.next();
            assert!(matches!(next, Some(Ok((_, $want, _)))), "{:?}", next);
        }};
    }

    #[test]
    fn it_lexes_names() {
        assert_token!("this_is_a_Name", Token::Name { .. });
        assert_token!("abcde", Token::Name { .. });
        assert_token!("a_b_cde_", Token::Name { .. });
        assert_token!("a123456789", Token::Name { .. });
        assert_token!("héllö", Token::Name { .. });
    }

    #[test]
    fn it_lexes_proper_names() {
        assert_token!("Abcde", Token::ProperName { .. });
        assert_token!("A_b_cde_", Token::ProperName { .. });
        assert_token!("A123456789", Token::ProperName { .. });
        assert_token!("ABCD", Token::ProperName { .. });
        assert_token!("Héllö", Token::ProperName { .. });
    }

    #[test]
    fn it_lexes_unsued_names() {
        assert_token!("_", Token::UnusedName { .. });
        assert_token!("_abcde", Token::UnusedName { .. });
        assert_token!("_a_b_cde_", Token::UnusedName { .. });
        assert_token!("_a123456789", Token::UnusedName { .. });
        assert_token!("_héllö", Token::UnusedName { .. });
    }

    #[test]
    fn it_lexes_package_names() {
        assert_token!("some-package3", Token::PackageName { .. });
    }

    #[test]
    fn it_lexes_integers() {
        assert_token!("5", Token::Int { .. });
        assert_token!("0", Token::Int { .. });
        assert_token!("123456789000000", Token::Int { .. });
        assert_token!("0005", Token::Int { .. });
        assert_token!("10_000_000", Token::Int { .. });
        assert_token!("--leading\n--leading0\n10 --trailing", Token::Int { .. });
    }

    #[test]
    fn it_lexes_floats() {
        assert_token!("5.0", Token::Float { .. });
        assert_token!("0.0", Token::Float { .. });
        assert_token!("5.0000", Token::Float { .. });
        assert_token!("123456789000000.123456", Token::Float { .. });
        assert_token!("1___2__3_.0___", Token::Float { .. });
        assert_token!(
            "--leading\n--leading0\n10.10 --trailing",
            Token::Float { .. }
        );
    }

    #[test]
    fn it_lexes_strings() {
        assert_token!(r#" "" "#, Token::String { .. });
        assert_token!(r#" "old school" "#, Token::String { .. });
        assert_token!(r#" " padded " "#, Token::String { .. });
        assert_token!(r#" "Hello, 世界" "#, Token::String { .. });
        assert_token!(r#" "👌🚀" "#, Token::String { .. });
        assert_token!(r#" "\n\r\t\"\\" "#, Token::String { .. });
    }

    #[test]
    fn it_errors_as_expected() {
        let mut lexer = Lexer::new("++");
        assert!(matches!(lexer.next(), Some(Err(Error::InvalidToken(_)))));
    }
}
