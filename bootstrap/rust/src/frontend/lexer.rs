use super::diagnostics::Diagnostic;
use super::source::{FileId, Span};
use super::token::{Keyword, Token, TokenKind};

pub type LexError = Diagnostic;
pub type LexResult = Result<Vec<Token>, Vec<LexError>>;

pub struct Lexer<'source> {
    file: FileId,
    source: &'source str,
    offset: usize,
    diagnostics: Vec<Diagnostic>,
    tokens: Vec<Token>,
}

impl<'source> Lexer<'source> {
    pub fn new(file: FileId, source: &'source str) -> Self {
        Self {
            file,
            source,
            offset: 0,
            diagnostics: Vec::new(),
            tokens: Vec::new(),
        }
    }

    pub fn lex(mut self) -> LexResult {
        while let Some(character) = self.peek() {
            let start = self.offset;
            match character {
                character if character.is_whitespace() => {
                    self.bump();
                }
                character if is_identifier_start(character) => self.lex_identifier(start),
                character if character.is_ascii_digit() => self.lex_number(start),
                '"' => self.lex_string(start),
                '\'' => self.lex_char(start),
                '(' => self.single(TokenKind::LeftParen),
                ')' => self.single(TokenKind::RightParen),
                '{' => self.single(TokenKind::LeftBrace),
                '}' => self.single(TokenKind::RightBrace),
                '[' => self.single(TokenKind::LeftBracket),
                ']' => self.single(TokenKind::RightBracket),
                ',' => self.single(TokenKind::Comma),
                ';' => self.single(TokenKind::Semicolon),
                ':' => self.single(TokenKind::Colon),
                '.' => self.single(TokenKind::Dot),
                '=' => self.one_or_two('=', TokenKind::EqualEqual, TokenKind::Equal),
                '!' => self.one_or_two('=', TokenKind::BangEqual, TokenKind::Bang),
                '<' => self.one_or_two('=', TokenKind::LessEqual, TokenKind::Less),
                '>' => self.one_or_two('=', TokenKind::GreaterEqual, TokenKind::Greater),
                '&' if self.peek_next() == Some('&') => self.double(TokenKind::AmpersandAmpersand),
                '|' if self.peek_next() == Some('|') => self.double(TokenKind::PipePipe),
                '+' => self.one_or_two('=', TokenKind::PlusEqual, TokenKind::Plus),
                '-' => self.one_or_two('=', TokenKind::MinusEqual, TokenKind::Minus),
                '*' => self.one_or_two('=', TokenKind::StarEqual, TokenKind::Star),
                '/' if self.peek_next() == Some('/') => self.lex_line_comment(),
                '/' if self.peek_next() == Some('*') => self.lex_block_comment(start),
                '/' => self.one_or_two('=', TokenKind::SlashEqual, TokenKind::Slash),
                _ => {
                    self.bump();
                    self.diagnostics.push(Diagnostic::error(
                        format!("unsupported character `{character}`"),
                        Span::new(self.file, start, self.offset),
                    ));
                }
            }
        }
        self.tokens.push(Token::new(
            TokenKind::Eof,
            Span::empty(self.file, self.offset),
        ));
        if self.diagnostics.is_empty() {
            Ok(self.tokens)
        } else {
            Err(self.diagnostics)
        }
    }

    fn peek(&self) -> Option<char> {
        self.source[self.offset..].chars().next()
    }

    fn peek_next(&self) -> Option<char> {
        let mut characters = self.source[self.offset..].chars();
        characters.next()?;
        characters.next()
    }

    fn bump(&mut self) -> Option<char> {
        let character = self.peek()?;
        self.offset += character.len_utf8();
        Some(character)
    }

    fn single(&mut self, kind: TokenKind) {
        let start = self.offset;
        self.bump();
        self.tokens
            .push(Token::new(kind, Span::new(self.file, start, self.offset)));
    }

    fn double(&mut self, kind: TokenKind) {
        let start = self.offset;
        self.bump();
        self.bump();
        self.tokens
            .push(Token::new(kind, Span::new(self.file, start, self.offset)));
    }

    fn one_or_two(&mut self, second: char, double: TokenKind, single: TokenKind) {
        if self.peek_next() == Some(second) {
            self.double(double);
        } else {
            self.single(single);
        }
    }

    fn lex_line_comment(&mut self) {
        self.bump();
        self.bump();
        while self
            .peek()
            .is_some_and(|character| character != '\n' && character != '\r')
        {
            self.bump();
        }
    }

    fn lex_block_comment(&mut self, start: usize) {
        self.bump();
        self.bump();
        while let Some(character) = self.bump() {
            if character == '*' && self.peek() == Some('/') {
                self.bump();
                return;
            }
        }
        self.diagnostics.push(Diagnostic::error(
            "unterminated block comment; expected `*/`",
            Span::new(self.file, start, self.offset),
        ));
    }

    fn lex_identifier(&mut self, start: usize) {
        self.bump();
        while self.peek().is_some_and(is_identifier_continue) {
            self.bump();
        }
        let text = &self.source[start..self.offset];
        let kind = Keyword::from_source(text)
            .map(TokenKind::Keyword)
            .unwrap_or_else(|| TokenKind::Identifier(text.to_owned()));
        self.tokens
            .push(Token::new(kind, Span::new(self.file, start, self.offset)));
    }

    fn lex_number(&mut self, start: usize) {
        while self
            .peek()
            .is_some_and(|character| character.is_ascii_digit())
        {
            self.bump();
        }
        let is_float = self.peek() == Some('.')
            && self.source[self.offset + 1..]
                .chars()
                .next()
                .is_some_and(|character| character.is_ascii_digit());
        if is_float {
            self.bump();
            while self
                .peek()
                .is_some_and(|character| character.is_ascii_digit())
            {
                self.bump();
            }
        }
        let text = self.source[start..self.offset].to_owned();
        self.tokens.push(Token::new(
            if is_float {
                TokenKind::Float(text)
            } else {
                TokenKind::Integer(text)
            },
            Span::new(self.file, start, self.offset),
        ));
    }

    fn lex_string(&mut self, start: usize) {
        self.bump();
        let mut value = String::new();
        while let Some(character) = self.peek() {
            if character == '"' {
                self.bump();
                self.tokens.push(Token::new(
                    TokenKind::String(value),
                    Span::new(self.file, start, self.offset),
                ));
                return;
            }
            if character == '\n' || character == '\r' {
                break;
            }
            self.bump();
            if character != '\\' {
                value.push(character);
                continue;
            }
            let Some(escape) = self.bump() else {
                break;
            };
            let escaped = match escape {
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                '0' => '\0',
                '\\' => '\\',
                '"' => '"',
                _ => {
                    self.diagnostics.push(Diagnostic::error(
                        format!("unsupported string escape `\\{escape}`"),
                        Span::new(self.file, self.offset - escape.len_utf8() - 1, self.offset),
                    ));
                    continue;
                }
            };
            value.push(escaped);
        }
        self.diagnostics.push(Diagnostic::error(
            "unterminated string literal",
            Span::new(self.file, start, self.offset),
        ));
    }

    fn lex_char(&mut self, start: usize) {
        self.bump();
        let Some(first) = self.bump() else {
            self.diagnostics.push(Diagnostic::error(
                "unterminated character literal",
                Span::new(self.file, start, self.offset),
            ));
            return;
        };
        let value = if first == '\\' {
            let Some(escape) = self.bump() else {
                self.diagnostics.push(Diagnostic::error(
                    "unterminated character escape",
                    Span::new(self.file, start, self.offset),
                ));
                return;
            };
            match escape {
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                '0' => '\0',
                '\\' => '\\',
                '\'' => '\'',
                _ => {
                    self.diagnostics.push(Diagnostic::error(
                        format!("unsupported character escape `\\{escape}`"),
                        Span::new(self.file, start, self.offset),
                    ));
                    return;
                }
            }
        } else {
            first
        };
        if (first != '\\' && (value == '\n' || value == '\r')) || self.peek() != Some('\'') {
            while self.peek().is_some_and(|character| {
                character != '\'' && character != '\n' && character != '\r'
            }) {
                self.bump();
            }
            if self.peek() == Some('\'') {
                self.bump();
            }
            self.diagnostics.push(Diagnostic::error(
                "character literals must contain exactly one character",
                Span::new(self.file, start, self.offset),
            ));
            return;
        }
        self.bump();
        self.tokens.push(Token::new(
            TokenKind::Char(value),
            Span::new(self.file, start, self.offset),
        ));
    }
}

// TODO(spec): Aerofyl's exact identifier alphabet is not documented. This small
// bootstrap recognizer accepts Unicode alphabetic characters, `_`, and numeric
// continuation characters so that named constructs can be represented at all.
fn is_identifier_start(character: char) -> bool {
    character == '_' || character.is_alphabetic()
}
fn is_identifier_continue(character: char) -> bool {
    is_identifier_start(character) || character.is_numeric()
}

pub fn lex(file: FileId, source: &str) -> LexResult {
    Lexer::new(file, source).lex()
}

#[cfg(test)]
mod tests {
    use super::super::token::Keyword;
    use super::*;

    fn kinds(source: &str) -> Vec<TokenKind> {
        lex(FileId(0), source)
            .unwrap()
            .into_iter()
            .map(|token| token.kind)
            .collect()
    }

    #[test]
    fn lexes_known_declaration_tokens() {
        assert_eq!(
            kinds("int[4] nums = [1, 2.5];"),
            vec![
                TokenKind::Keyword(Keyword::Int),
                TokenKind::LeftBracket,
                TokenKind::Integer("4".into()),
                TokenKind::RightBracket,
                TokenKind::Identifier("nums".into()),
                TokenKind::Equal,
                TokenKind::LeftBracket,
                TokenKind::Integer("1".into()),
                TokenKind::Comma,
                TokenKind::Float("2.5".into()),
                TokenKind::RightBracket,
                TokenKind::Semicolon,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lexes_required_integer_operators() {
        assert_eq!(
            kinds("+ - * /"),
            vec![
                TokenKind::Plus,
                TokenKind::Minus,
                TokenKind::Star,
                TokenKind::Slash,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lexes_control_flow_boolean_and_comparison_tokens() {
        assert_eq!(
            kinds("if else while break continue true false == != < <= > >= ! && || ="),
            vec![
                TokenKind::Keyword(Keyword::If),
                TokenKind::Keyword(Keyword::Else),
                TokenKind::Keyword(Keyword::While),
                TokenKind::Keyword(Keyword::Break),
                TokenKind::Keyword(Keyword::Continue),
                TokenKind::Keyword(Keyword::True),
                TokenKind::Keyword(Keyword::False),
                TokenKind::EqualEqual,
                TokenKind::BangEqual,
                TokenKind::Less,
                TokenKind::LessEqual,
                TokenKind::Greater,
                TokenKind::GreaterEqual,
                TokenKind::Bang,
                TokenKind::AmpersandAmpersand,
                TokenKind::PipePipe,
                TokenKind::Equal,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lexes_unicode_char_literal() {
        assert_eq!(kinds("'λ'"), vec![TokenKind::Char('λ'), TokenKind::Eof]);
    }

    #[test]
    fn lexes_required_character_escapes() {
        assert_eq!(
            kinds("'\\n' '\\t' '\\r' '\\0' '\\\\' '\\\''"),
            vec![
                TokenKind::Char('\n'),
                TokenKind::Char('\t'),
                TokenKind::Char('\r'),
                TokenKind::Char('\0'),
                TokenKind::Char('\\'),
                TokenKind::Char('\''),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn skips_line_and_block_comments() {
        assert_eq!(
            kinds("int // line\n/* block\ncomment */ value"),
            vec![
                TokenKind::Keyword(Keyword::Int),
                TokenKind::Identifier("value".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn decodes_required_string_escapes() {
        assert_eq!(
            kinds("\"a\\n\\r\\t\\0\\\\\\\"b\""),
            vec![TokenKind::String("a\n\r\t\0\\\"b".into()), TokenKind::Eof]
        );
    }

    #[test]
    fn diagnoses_unterminated_block_comments_and_unknown_string_escapes() {
        let comment = lex(FileId(0), "/* missing").unwrap_err();
        assert!(comment[0].message.contains("unterminated block comment"));
        let escape = lex(FileId(0), "\"bad\\q\"").unwrap_err();
        assert!(escape[0].message.contains("unsupported string escape"));
    }
}
