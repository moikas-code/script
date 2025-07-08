use crate::source::Span;
use ahash::AHashMap;
use std::fmt;
use std::sync::OnceLock;

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
    pub lexeme: String,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span, lexeme: impl Into<String>) -> Self {
        Self {
            kind,
            span,
            lexeme: lexeme.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals
    Number(f64),
    String(String),
    Identifier(String),

    // Keywords
    Fn,
    Let,
    Mut,
    If,
    Else,
    While,
    For,
    Return,
    True,
    False,
    Print,
    Match,
    Async,
    Await,
    Struct,
    Enum,
    Impl,
    Where,

    // Module system keywords
    Import,
    Export,
    From,
    As,

    // Metaprogramming keywords
    In, // for list comprehensions

    // Operators
    At, // @ symbol for attributes
    Plus,
    Minus,
    Star,
    Slash,
    Percent,

    // Assignment
    Equals,

    // Comparison
    EqualsEquals,
    BangEquals,
    Less,
    Greater,
    LessEquals,
    GreaterEquals,

    // Logical
    And,
    Or,
    Bang,
    Pipe, // | for pattern matching
    Ampersand, // & for reference types

    // Delimiters
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Comma,
    Dot,
    Semicolon,
    Colon,
    ColonColon,
    Arrow,
    DoubleArrow,
    DotDot,
    Underscore,
    Question, // ? for error propagation

    // Special
    DocComment(String), // Documentation comments (/// or /** */)
    Newline,
    Eof,
}

impl TokenKind {
    /// Get the keyword map (initialized once using OnceLock for thread safety)
    fn keyword_map() -> &'static AHashMap<&'static str, TokenKind> {
        static KEYWORD_MAP: OnceLock<AHashMap<&'static str, TokenKind>> = OnceLock::new();
        KEYWORD_MAP.get_or_init(|| {
            let mut map = AHashMap::new();
            map.insert("fn", TokenKind::Fn);
            map.insert("let", TokenKind::Let);
            map.insert("mut", TokenKind::Mut);
            map.insert("if", TokenKind::If);
            map.insert("else", TokenKind::Else);
            map.insert("while", TokenKind::While);
            map.insert("for", TokenKind::For);
            map.insert("return", TokenKind::Return);
            map.insert("true", TokenKind::True);
            map.insert("false", TokenKind::False);
            map.insert("match", TokenKind::Match);
            map.insert("async", TokenKind::Async);
            map.insert("await", TokenKind::Await);
            map.insert("struct", TokenKind::Struct);
            map.insert("enum", TokenKind::Enum);
            map.insert("impl", TokenKind::Impl);
            map.insert("where", TokenKind::Where);
            map.insert("import", TokenKind::Import);
            map.insert("export", TokenKind::Export);
            map.insert("from", TokenKind::From);
            map.insert("as", TokenKind::As);
            map.insert("in", TokenKind::In);
            // "print" is a built-in function, not a keyword
            map
        })
    }
    
    /// O(1) keyword lookup using hash map
    pub fn from_keyword(word: &str) -> Option<Self> {
        Self::keyword_map().get(word).cloned()
    }
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::Number(n) => write!(f, "Number({})", n),
            TokenKind::String(s) => write!(f, "String(\"{}\")", s),
            TokenKind::Identifier(id) => write!(f, "Identifier({})", id),

            TokenKind::Fn => write!(f, "fn"),
            TokenKind::Let => write!(f, "let"),
            TokenKind::Mut => write!(f, "mut"),
            TokenKind::If => write!(f, "if"),
            TokenKind::Else => write!(f, "else"),
            TokenKind::While => write!(f, "while"),
            TokenKind::For => write!(f, "for"),
            TokenKind::Return => write!(f, "return"),
            TokenKind::True => write!(f, "true"),
            TokenKind::False => write!(f, "false"),
            TokenKind::Print => write!(f, "print"),
            TokenKind::Match => write!(f, "match"),
            TokenKind::Async => write!(f, "async"),
            TokenKind::Await => write!(f, "await"),
            TokenKind::Struct => write!(f, "struct"),
            TokenKind::Enum => write!(f, "enum"),
            TokenKind::Impl => write!(f, "impl"),
            TokenKind::Where => write!(f, "where"),

            TokenKind::Import => write!(f, "import"),
            TokenKind::Export => write!(f, "export"),
            TokenKind::From => write!(f, "from"),
            TokenKind::As => write!(f, "as"),
            TokenKind::In => write!(f, "in"),

            TokenKind::At => write!(f, "@"),
            TokenKind::Plus => write!(f, "+"),
            TokenKind::Minus => write!(f, "-"),
            TokenKind::Star => write!(f, "*"),
            TokenKind::Slash => write!(f, "/"),
            TokenKind::Percent => write!(f, "%"),

            TokenKind::Equals => write!(f, "="),
            TokenKind::EqualsEquals => write!(f, "=="),
            TokenKind::BangEquals => write!(f, "!="),
            TokenKind::Less => write!(f, "<"),
            TokenKind::Greater => write!(f, ">"),
            TokenKind::LessEquals => write!(f, "<="),
            TokenKind::GreaterEquals => write!(f, ">="),

            TokenKind::And => write!(f, "&&"),
            TokenKind::Or => write!(f, "||"),
            TokenKind::Bang => write!(f, "!"),
            TokenKind::Pipe => write!(f, "|"),
            TokenKind::Ampersand => write!(f, "&"),

            TokenKind::LeftParen => write!(f, "("),
            TokenKind::RightParen => write!(f, ")"),
            TokenKind::LeftBrace => write!(f, "{{"),
            TokenKind::RightBrace => write!(f, "}}"),
            TokenKind::LeftBracket => write!(f, "["),
            TokenKind::RightBracket => write!(f, "]"),
            TokenKind::Comma => write!(f, ","),
            TokenKind::Dot => write!(f, "."),
            TokenKind::Semicolon => write!(f, ";"),
            TokenKind::Colon => write!(f, ":"),
            TokenKind::ColonColon => write!(f, "::"),
            TokenKind::Arrow => write!(f, "->"),
            TokenKind::DoubleArrow => write!(f, "=>"),
            TokenKind::DotDot => write!(f, ".."),
            TokenKind::Underscore => write!(f, "_"),
            TokenKind::Question => write!(f, "?"),

            TokenKind::DocComment(s) => write!(f, "DocComment(\"{}\")", s),
            TokenKind::Newline => write!(f, "\\n"),
            TokenKind::Eof => write!(f, "EOF"),
        }
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} @ {}", self.kind, self.span)
    }
}
