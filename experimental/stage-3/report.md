# REPORT - 1

## Results

* finished token.fyl
* finished source.fyl
* finished diagnostics.fyl

## Walls

> LEVEL: MEDIUM
1. Currently we dont have `using` statement meaning i had to import parts of the code fully instead of a small part that i needed.

> LEVEL: TINY
2. No `impl` statement means i will need to modify parts of the code to adapt

# REPORT - 2 LEXER.FYL

## REUSLTS

## WALLS














































### Code:

token.fyl

```AeroFyl
/*
This an experimental token.fyl for Stage-3 of bootstrap,
I wiil use comments point out missing features
*/
//because we dont have using yet i have to load the whole source.fyl
use source;

public enum Keyword {
    Public,
    Private,
    Use,
    Using,
    Return,
    If,
    Else,
    While,
    For,
    Break,
    Continue,
    True,
    False,
    Print,
    Input,
    List,
    Optional,
    Ref,
    Int,
    Byte,
    Float,
    Bool,
    Char,
    String,
    Void,
    Dynamic,
    Struct,
    Enum
}
//In the future i will do smth like impl then fn from_source like orignal token.rs

public enum TokenKind {
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
    PlusEqual,
    Minus,
    MinusEqual,
    Star,
    StarEqual,
    Slash,
    SlashEqual,
    Eof,
}

pub struct token {
    TokenKind kind;
    Span span;
}
```

diagnostics.fyl
```AeroFyl

//same problem as token.fyl i dont have using yet
use source;

enum DiagnosticLevel {
    Error,
    Warning,
    Note,
}

struct Diagnostic {
    DiagnosticLevel level;
    string message;
    Span span;
}

public Diagnostic error(string message, Span span)
{
    return Diagnostic {
        level: DiagnosticLevel.Error,
        message: message,
        span: span
    };
}
```

source.fyl
```
/*
To whoever is reading this - i want to eat but i am forced
to work on this experimental compiler. please give me food
*/


use std.fs;


//File reading:
struct Source {
    string path;
    string context;
}
public Source load_file(string path){
    return Source {
        path: path,
        context: readFile(path)
    };
}
/*--------*/

//span
stuct Span {
    int start;
    int end;
}
/*--------*/
```