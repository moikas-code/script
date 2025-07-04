use super::{Token, TokenKind};
use crate::{
    error::{Error, Result},
    source::{SourceLocation, Span},
};

pub struct Lexer {
    input: Vec<char>,
    current: usize,
    location: SourceLocation,
    start_location: SourceLocation,
    start_index: usize,
    tokens: Vec<Token>,
    errors: Vec<Error>,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            current: 0,
            location: SourceLocation::initial(),
            start_location: SourceLocation::initial(),
            start_index: 0,
            tokens: Vec::new(),
            errors: Vec::new(),
        }
    }

    pub fn scan_tokens(mut self) -> (Vec<Token>, Vec<Error>) {
        while !self.is_at_end() {
            self.start_location = self.location;
            self.start_index = self.current;
            self.scan_token();
        }

        self.add_token(TokenKind::Eof);
        (self.tokens, self.errors)
    }

    /// Get the next token for LSP and iterative parsing
    /// Returns the token and any errors encountered
    pub fn next_token(&mut self) -> (Option<Token>, Vec<Error>) {
        // Clear previous errors for this token
        self.errors.clear();

        if self.is_at_end() {
            // Return EOF token
            self.start_location = self.location;
            self.start_index = self.current;
            self.add_token(TokenKind::Eof);
            return (self.tokens.pop(), self.errors.clone());
        }

        // Set up for scanning the next token
        self.start_location = self.location;
        self.start_index = self.current;

        // Scan the token
        self.scan_token();

        // Return the last token added and any errors
        (self.tokens.pop(), self.errors.clone())
    }

    fn scan_token(&mut self) {
        let c = self.advance();
        match c {
            // Whitespace
            ' ' | '\r' | '\t' => {}
            '\n' => {
                // Newlines can be significant in Script for statement termination
                self.add_token(TokenKind::Newline);
            }

            // Single character tokens
            '(' => self.add_token(TokenKind::LeftParen),
            ')' => self.add_token(TokenKind::RightParen),
            '{' => self.add_token(TokenKind::LeftBrace),
            '}' => self.add_token(TokenKind::RightBrace),
            '[' => self.add_token(TokenKind::LeftBracket),
            ']' => self.add_token(TokenKind::RightBracket),
            ',' => self.add_token(TokenKind::Comma),
            ';' => self.add_token(TokenKind::Semicolon),
            ':' => {
                if self.match_char(':') {
                    self.add_token(TokenKind::ColonColon);
                } else {
                    self.add_token(TokenKind::Colon);
                }
            }
            '+' => self.add_token(TokenKind::Plus),
            '*' => self.add_token(TokenKind::Star),
            '%' => self.add_token(TokenKind::Percent),

            // Two character tokens
            '-' => {
                if self.match_char('>') {
                    self.add_token(TokenKind::Arrow);
                } else {
                    self.add_token(TokenKind::Minus);
                }
            }
            '!' => {
                if self.match_char('=') {
                    self.add_token(TokenKind::BangEquals);
                } else {
                    self.add_token(TokenKind::Bang);
                }
            }
            '=' => {
                if self.match_char('=') {
                    self.add_token(TokenKind::EqualsEquals);
                } else if self.match_char('>') {
                    self.add_token(TokenKind::DoubleArrow);
                } else {
                    self.add_token(TokenKind::Equals);
                }
            }
            '<' => {
                if self.match_char('=') {
                    self.add_token(TokenKind::LessEquals);
                } else {
                    self.add_token(TokenKind::Less);
                }
            }
            '>' => {
                if self.match_char('=') {
                    self.add_token(TokenKind::GreaterEquals);
                } else {
                    self.add_token(TokenKind::Greater);
                }
            }
            '&' => {
                if self.match_char('&') {
                    self.add_token(TokenKind::And);
                } else {
                    self.error("Unexpected character '&', did you mean '&&'?");
                }
            }
            '|' => {
                if self.match_char('|') {
                    self.add_token(TokenKind::Or);
                } else {
                    self.add_token(TokenKind::Pipe);
                }
            }
            '.' => {
                if self.match_char('.') {
                    self.add_token(TokenKind::DotDot);
                } else {
                    self.add_token(TokenKind::Dot);
                }
            }

            // Comments
            '/' => {
                if self.match_char('/') {
                    // Check if it's a doc comment (///)
                    if self.match_char('/') {
                        self.scan_doc_comment();
                    } else {
                        // Regular single line comment
                        while self.peek() != '\n' && !self.is_at_end() {
                            self.advance();
                        }
                    }
                } else if self.match_char('*') {
                    // Check if it's a doc comment (/**)
                    if self.peek() == '*' && self.peek_next() != '/' {
                        self.scan_multiline_doc_comment();
                    } else {
                        // Regular multi-line comment
                        self.scan_multiline_comment();
                    }
                } else {
                    self.add_token(TokenKind::Slash);
                }
            }

            // String literals
            '"' => self.scan_string(),

            // Numbers
            '0'..='9' => self.scan_number(),

            // Identifiers and keywords
            'a'..='z' | 'A'..='Z' | '_' => self.scan_identifier(),

            // Metaprogramming
            '@' => self.add_token(TokenKind::At),

            // Unknown character
            _ => self.error(&format!("Unexpected character: '{}'", c)),
        }
    }

    fn scan_multiline_comment(&mut self) {
        let mut nesting = 1;

        while nesting > 0 && !self.is_at_end() {
            if self.peek() == '/' && self.peek_next() == '*' {
                self.advance();
                self.advance();
                nesting += 1;
            } else if self.peek() == '*' && self.peek_next() == '/' {
                self.advance();
                self.advance();
                nesting -= 1;
            } else {
                self.advance();
            }
        }

        if nesting > 0 {
            self.error("Unterminated comment");
        }
    }

    fn scan_string(&mut self) {
        let mut value = String::new();

        while self.peek() != '"' && !self.is_at_end() {
            if self.peek() == '\n' {
                self.error("Unterminated string");
                return;
            }

            if self.peek() == '\\' {
                self.advance();
                let escaped = match self.peek() {
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    '\\' => '\\',
                    '"' => '"',
                    _ => {
                        self.error(&format!("Invalid escape sequence: \\{}", self.peek()));
                        self.peek()
                    }
                };
                value.push(escaped);
                self.advance();
            } else {
                value.push(self.advance());
            }
        }

        if self.is_at_end() {
            self.error("Unterminated string");
            return;
        }

        // Consume closing "
        self.advance();

        self.add_token(TokenKind::String(value));
    }

    fn scan_number(&mut self) {
        while self.peek().is_ascii_digit() {
            self.advance();
        }

        // Look for decimal part
        if self.peek() == '.' && self.peek_next().is_ascii_digit() {
            // Consume the .
            self.advance();

            while self.peek().is_ascii_digit() {
                self.advance();
            }
        }

        let value: String = self.input[self.start_index..self.current].iter().collect();

        match value.parse::<f64>() {
            Ok(num) => self.add_token(TokenKind::Number(num)),
            Err(_) => self.error(&format!("Invalid number: {}", value)),
        }
    }

    fn scan_identifier(&mut self) {
        while self.peek().is_alphanumeric() || self.peek() == '_' {
            self.advance();
        }

        let value: String = self.input[self.start_index..self.current].iter().collect();

        // Special case: standalone underscore is a wildcard token
        if value == "_" {
            self.add_token(TokenKind::Underscore);
        } else {
            let token_kind = TokenKind::from_keyword(&value)
                .unwrap_or_else(|| TokenKind::Identifier(value.clone()));
            self.add_token(token_kind);
        }
    }

    fn advance(&mut self) -> char {
        let ch = self.input[self.current];
        self.current += 1;
        self.location.advance(ch);
        ch
    }

    fn match_char(&mut self, expected: char) -> bool {
        if self.is_at_end() || self.input[self.current] != expected {
            false
        } else {
            self.advance();
            true
        }
    }

    fn peek(&self) -> char {
        if self.is_at_end() {
            '\0'
        } else {
            self.input[self.current]
        }
    }

    fn peek_next(&self) -> char {
        if self.current + 1 >= self.input.len() {
            '\0'
        } else {
            self.input[self.current + 1]
        }
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.input.len()
    }

    fn add_token(&mut self, kind: TokenKind) {
        let span = Span::new(self.start_location, self.location);
        let lexeme: String = self.input[self.start_index..self.current].iter().collect();

        self.tokens.push(Token::new(kind, span, lexeme));
    }

    fn scan_doc_comment(&mut self) {
        let mut content = String::new();

        // Skip any additional slashes
        while self.peek() == '/' {
            self.advance();
        }

        // Skip initial whitespace
        if self.peek() == ' ' {
            self.advance();
        }

        // Collect the rest of the line
        while self.peek() != '\n' && !self.is_at_end() {
            content.push(self.advance());
        }

        self.add_token(TokenKind::DocComment(content.trim().to_string()));
    }

    fn scan_multiline_doc_comment(&mut self) {
        let mut content = String::new();
        let mut first_line = true;

        // Advance past the initial *
        self.advance();

        // Skip whitespace after /**
        if self.peek() == ' ' {
            self.advance();
        }

        while !self.is_at_end() {
            // Check for end of comment
            if self.peek() == '*' && self.peek_next() == '/' {
                self.advance(); // consume *
                self.advance(); // consume /
                break;
            }

            let ch = self.advance();

            if ch == '\n' {
                // Trim trailing whitespace from the line
                let trimmed = content.trim_end().to_string();
                content.clear();
                content.push_str(&trimmed);
                content.push('\n');
                first_line = false;

                // Skip leading whitespace and * on continuation lines
                while self.peek() == ' ' || self.peek() == '\t' {
                    self.advance();
                }

                if self.peek() == '*' && self.peek_next() != '/' {
                    self.advance(); // Skip the *
                    if self.peek() == ' ' {
                        self.advance(); // Skip space after *
                    }
                }
            } else {
                content.push(ch);
            }
        }

        // Remove trailing newline if present
        let trimmed = content.trim().to_string();
        self.add_token(TokenKind::DocComment(trimmed));
    }

    fn error(&mut self, message: &str) {
        self.errors
            .push(Error::lexer(message).with_location(self.location));
    }
}

// Iterator implementation for convenient usage
impl IntoIterator for Lexer {
    type Item = Result<Token>;
    type IntoIter = LexerIterator;

    fn into_iter(self) -> Self::IntoIter {
        let (tokens, errors) = self.scan_tokens();
        LexerIterator {
            tokens: tokens.into_iter(),
            errors: errors.into_iter(),
        }
    }
}

pub struct LexerIterator {
    tokens: std::vec::IntoIter<Token>,
    errors: std::vec::IntoIter<Error>,
}

impl Iterator for LexerIterator {
    type Item = Result<Token>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(error) = self.errors.next() {
            Some(Err(error))
        } else {
            self.tokens.next().map(Ok)
        }
    }
}
