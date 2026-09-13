use super::source::Span;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Keyword {
    Public,
    Private,
    Use,
    Using,
    Return,
    If,
    Else,
    While,
    Break,
    Continue,
    True,
    False,
    Print,
    Input,
    List,
    Int,
    Float,
    Bool,
    Char,
    String,
    Void,
    Dynamic,
    Struct,
    Enum,
}

impl Keyword {
    pub fn from_source(text: &str) -> Option<Self> {
        Some(match text {
            "public" => Self::Public,
            "private" => Self::Private,
            "use" => Self::Use,
            "using" => Self::Using,
            "return" => Self::Return,
            "if" => Self::If,
            "else" => Self::Else,
            "while" => Self::While,
            "break" => Self::Break,
            "continue" => Self::Continue,
            "true" => Self::True,
            "false" => Self::False,
            "print" => Self::Print,
            "input" => Self::Input,
            "list" => Self::List,
            "int" => Self::Int,
            "float" => Self::Float,
            "bool" => Self::Bool,
            "char" => Self::Char,
            "string" => Self::String,
            "void" => Self::Void,
            "dynamic" => Self::Dynamic,
            "struct" => Self::Struct,
            "enum" => Self::Enum,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind {
    Identifier(String),
    Integer(String),
    Float(String),
    String(String),
    Char(char),
    Keyword(Keyword),
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Comma,
    Semicolon,
    Colon,
    Dot,
    Equal,
    EqualEqual,
    BangEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Bang,
    AmpersandAmpersand,
    PipePipe,
    Plus,
    Minus,
    Star,
    Slash,
    Eof,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub const fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}
