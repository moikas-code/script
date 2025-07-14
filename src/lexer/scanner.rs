use super::lru_cache::LruCache;
use super::{Token, TokenKind};
use crate::{
    error::{Error, Result},
    source::{SourceLocation, Span},
};
use ahash::{AHashMap, AHashSet};
use unicode_normalization::UnicodeNormalization;

// Type aliases for fast hash collections
type HashMap<K, V> = AHashMap<K, V>;
type HashSet<T> = AHashSet<T>;

// Security limits to prevent DoS attacks
const MAX_INPUT_SIZE: usize = 1024 * 1024; // 1MB max input
const MAX_STRING_LITERAL_SIZE: usize = 64 * 1024; // 64KB max string literal
const MAX_COMMENT_NESTING_DEPTH: u32 = 32; // 32 levels max nesting
const MAX_TOKEN_COUNT: usize = 100_000; // 100K tokens max

// Cache size limits to prevent memory exhaustion
const MAX_CACHE_ENTRIES: usize = 10_000; // Maximum entries per cache
const MAX_STRING_INTERNER_SIZE: usize = 50_000; // Maximum interned strings

// Use production-safe error messages that don't expose internal details
#[cfg(not(debug_assertions))]
const PRODUCTION_ERRORS: bool = true;
#[cfg(debug_assertions)]
const PRODUCTION_ERRORS: bool = false;

/// Unicode security configuration levels
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnicodeSecurityLevel {
    /// Strict: Reject confusable identifiers, normalize all Unicode
    Strict,
    /// Warning: Warn about confusable identifiers, normalize all Unicode  
    Warning,
    /// Permissive: Allow confusable identifiers, normalize all Unicode
    Permissive,
}

impl Default for UnicodeSecurityLevel {
    fn default() -> Self {
        Self::Warning
    }
}

/// Unicode security configuration
#[derive(Debug, Clone)]
pub struct UnicodeSecurityConfig {
    pub level: UnicodeSecurityLevel,
    pub normalize_identifiers: bool,
    pub detect_confusables: bool,
}

impl Default for UnicodeSecurityConfig {
    fn default() -> Self {
        Self {
            level: UnicodeSecurityLevel::Warning,
            normalize_identifiers: true,
            detect_confusables: true,
        }
    }
}

/// Cache for Unicode processing to improve performance
#[derive(Debug)]
struct UnicodeCache {
    /// Cache for normalized identifier strings
    normalization_cache: LruCache<String, String>,
    /// Cache for confusable skeletons
    skeleton_cache: LruCache<String, String>,
    /// Set of known confusable identifiers that have been warned about
    warned_confusables: HashSet<String>,
}

impl UnicodeCache {
    fn new() -> Self {
        Self {
            normalization_cache: LruCache::new(MAX_CACHE_ENTRIES),
            skeleton_cache: LruCache::new(MAX_CACHE_ENTRIES),
            warned_confusables: HashSet::new(),
        }
    }
}

/// String interning system for efficient memory usage
#[derive(Debug)]
struct StringInterner {
    strings: Vec<String>,
    string_map: HashMap<String, usize>,
}

impl StringInterner {
    fn new() -> Self {
        Self {
            strings: Vec::new(),
            string_map: HashMap::new(),
        }
    }

    /// Intern a string and return its index
    fn intern(&mut self, s: String) -> usize {
        if let Some(&index) = self.string_map.get(&s) {
            index
        } else {
            // Check size limit to prevent memory exhaustion
            if self.strings.len() >= MAX_STRING_INTERNER_SIZE {
                // Return index 0 as fallback (should have empty string)
                // In production, this should be handled more gracefully
                return 0;
            }
            let index = self.strings.len();
            self.string_map.insert(s.clone(), index);
            self.strings.push(s);
            index
        }
    }

    /// Get a string by its index
    fn get(&self, index: usize) -> Option<&str> {
        self.strings.get(index).map(|s| s.as_str())
    }
}

pub struct Lexer {
    input: String,
    input_bytes: Vec<u8>,
    current: usize,      // Byte offset
    current_char: usize, // Character offset for location tracking
    location: SourceLocation,
    start_location: SourceLocation,
    start_index: usize,      // Byte offset
    start_char_index: usize, // Character offset
    tokens: Vec<Token>,
    errors: Vec<Error>,
    string_interner: StringInterner,
    unicode_config: UnicodeSecurityConfig,
    unicode_cache: UnicodeCache,
}

impl Lexer {
    pub fn new(input: &str) -> Result<Self> {
        Self::with_unicode_config(input, UnicodeSecurityConfig::default())
    }

    pub fn with_unicode_config(input: &str, unicode_config: UnicodeSecurityConfig) -> Result<Self> {
        // Check input size limit
        if input.len() > MAX_INPUT_SIZE {
            if PRODUCTION_ERRORS {
                return Err(Error::lexer("Input exceeds maximum allowed size"));
            } else {
                return Err(Error::lexer(&format!(
                    "Input size {} exceeds maximum allowed size of {} bytes",
                    input.len(),
                    MAX_INPUT_SIZE
                )));
            }
        }

        Ok(Self {
            input: input.to_string(),
            input_bytes: input.as_bytes().to_vec(),
            current: 0,
            current_char: 0,
            location: SourceLocation::initial(),
            start_location: SourceLocation::initial(),
            start_index: 0,
            start_char_index: 0,
            tokens: Vec::new(),
            errors: Vec::new(),
            string_interner: StringInterner::new(),
            unicode_config,
            unicode_cache: UnicodeCache::new(),
        })
    }

    /// Get the current Unicode security configuration
    pub fn unicode_config(&self) -> &UnicodeSecurityConfig {
        &self.unicode_config
    }

    /// Set the Unicode security configuration
    pub fn set_unicode_config(&mut self, config: UnicodeSecurityConfig) {
        self.unicode_config = config;
    }

    pub fn scan_tokens(mut self) -> (Vec<Token>, Vec<Error>) {
        while !self.is_at_end() {
            self.start_location = self.location;
            self.start_index = self.current;
            self.start_char_index = self.current_char;
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
            self.start_char_index = self.current_char;
            self.add_token(TokenKind::Eof);
            return (self.tokens.pop(), self.errors.clone());
        }

        // Set up for scanning the next token
        self.start_location = self.location;
        self.start_index = self.current;
        self.start_char_index = self.current_char;

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
                    self.add_token(TokenKind::Ampersand);
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

            // Error propagation
            '?' => self.add_token(TokenKind::Question),

            // Unknown character
            _ => self.error("Unexpected character"),
        }
    }

    fn scan_multiline_comment(&mut self) {
        let mut nesting = 1u32;

        while nesting > 0 && !self.is_at_end() {
            if self.peek() == '/' && self.peek_next() == '*' {
                self.advance();
                self.advance();
                nesting = match nesting.checked_add(1) {
                    Some(new_nesting) if new_nesting <= MAX_COMMENT_NESTING_DEPTH => new_nesting,
                    _ => {
                        self.error_with_details(
                            "Comment nesting exceeds maximum allowed depth",
                            &format!(
                                "Comment nesting depth exceeds maximum allowed depth of {}",
                                MAX_COMMENT_NESTING_DEPTH
                            ),
                        );
                        return;
                    }
                };
            } else if self.peek() == '*' && self.peek_next() == '/' {
                self.advance();
                self.advance();
                nesting = nesting.saturating_sub(1);
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
            // Check string length limit
            if value.len() >= MAX_STRING_LITERAL_SIZE {
                self.error_with_details(
                    "String literal exceeds maximum allowed size",
                    &format!(
                        "String literal length {} exceeds maximum allowed size of {} bytes",
                        value.len(),
                        MAX_STRING_LITERAL_SIZE
                    ),
                );
                return;
            }

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
                        self.error("Invalid escape sequence");
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

        // Intern string literal for memory efficiency
        let interned_index = self.string_interner.intern(value);
        if let Some(interned_string) = self.string_interner.get(interned_index) {
            self.add_token(TokenKind::String(interned_string.to_string()));
        }
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

        let value: String = if self.start_index <= self.current && self.current <= self.input.len()
        {
            self.input[self.start_index..self.current].to_string()
        } else {
            String::new() // Safe fallback for invalid indices
        };

        match value.parse::<f64>() {
            Ok(num) => self.add_token(TokenKind::Number(num)),
            Err(_) => self.error("Invalid number format"),
        }
    }

    /// Normalize an identifier using Unicode NFKC normalization
    fn normalize_identifier(&mut self, identifier: &str) -> String {
        // Fast path for ASCII-only identifiers (common case)
        if identifier.is_ascii() {
            return identifier.to_string();
        }

        // Check cache first
        if let Some(normalized) = self
            .unicode_cache
            .normalization_cache
            .get(&identifier.to_string())
        {
            return normalized;
        }

        // Perform NFKC normalization
        let normalized: String = identifier.nfkc().collect();

        // Cache the result for performance
        self.unicode_cache
            .normalization_cache
            .insert(identifier.to_string(), normalized.clone());

        normalized
    }

    /// Enhanced confusable detection with more comprehensive coverage
    fn simple_confusable_skeleton(&self, s: &str) -> String {
        s.chars()
            .map(|c| match c {
                // Latin/Cyrillic confusables
                'а' => 'a', // Cyrillic small letter a -> Latin a
                'А' => 'A', // Cyrillic capital letter a -> Latin A
                'е' => 'e', // Cyrillic small letter e -> Latin e
                'Е' => 'E', // Cyrillic capital letter e -> Latin E
                'о' => 'o', // Cyrillic small letter o -> Latin o
                'О' => 'O', // Cyrillic capital letter o -> Latin O
                'р' => 'p', // Cyrillic small letter p -> Latin p
                'Р' => 'P', // Cyrillic capital letter p -> Latin P
                'с' => 'c', // Cyrillic small letter c -> Latin c
                'С' => 'C', // Cyrillic capital letter c -> Latin C
                'х' => 'x', // Cyrillic small letter x -> Latin x
                'Х' => 'X', // Cyrillic capital letter x -> Latin X
                'у' => 'y', // Cyrillic small letter y -> Latin y
                'У' => 'Y', // Cyrillic capital letter y -> Latin Y

                // Greek confusables
                'α' => 'a', // Greek small letter alpha -> Latin a
                'Α' => 'A', // Greek capital letter alpha -> Latin A
                'ε' => 'e', // Greek small letter epsilon -> Latin e
                'Ε' => 'E', // Greek capital letter epsilon -> Latin E
                'ο' => 'o', // Greek small letter omicron -> Latin o
                'Ο' => 'O', // Greek capital letter omicron -> Latin O
                'ρ' => 'p', // Greek small letter rho -> Latin p
                'Ρ' => 'P', // Greek capital letter rho -> Latin P
                'χ' => 'x', // Greek small letter chi -> Latin x
                'Χ' => 'X', // Greek capital letter chi -> Latin X
                'υ' => 'y', // Greek small letter upsilon -> Latin y
                'Υ' => 'Y', // Greek capital letter upsilon -> Latin Y

                // Mathematical Alphanumeric Symbols
                '𝐚'..='𝐳' => (b'a' + (c as u32 - '𝐚' as u32) as u8) as char,
                '𝐀'..='𝐙' => (b'A' + (c as u32 - '𝐀' as u32) as u8) as char,
                '𝑎'..='𝑧' => (b'a' + (c as u32 - '𝑎' as u32) as u8) as char,
                '𝐴'..='𝑍' => (b'A' + (c as u32 - '𝐴' as u32) as u8) as char,
                '𝒂'..='𝒛' => (b'a' + (c as u32 - '𝒂' as u32) as u8) as char,
                '𝑨'..='𝒁' => (b'A' + (c as u32 - '𝑨' as u32) as u8) as char,

                // Fullwidth Forms
                'ａ'..='ｚ' => (b'a' + (c as u32 - 'ａ' as u32) as u8) as char,
                'Ａ'..='Ｚ' => (b'A' + (c as u32 - 'Ａ' as u32) as u8) as char,
                '０'..='９' => (b'0' + (c as u32 - '０' as u32) as u8) as char,

                // Subscript and Superscript
                '₀'..='₉' => (b'0' + (c as u32 - '₀' as u32) as u8) as char,
                '⁰' => '0',
                '¹' => '1',
                '²' => '2',
                '³' => '3',
                '⁴' => '4',
                '⁵' => '5',
                '⁶' => '6',
                '⁷' => '7',
                '⁸' => '8',
                '⁹' => '9',

                // Small Form Variants
                'ᴀ' => 'A',
                'ʙ' => 'B',
                'ᴄ' => 'C',
                'ᴅ' => 'D',
                'ᴇ' => 'E',
                'ꜰ' => 'F',
                'ɢ' => 'G',
                'ʜ' => 'H',
                'ɪ' => 'I',
                'ᴊ' => 'J',
                'ᴋ' => 'K',
                'ʟ' => 'L',
                'ᴍ' => 'M',
                'ɴ' => 'N',
                'ᴏ' => 'O',
                'ᴘ' => 'P',
                'ʀ' => 'R',
                'ꜱ' => 'S',
                'ᴛ' => 'T',
                'ᴜ' => 'U',
                'ᴠ' => 'V',
                'ᴡ' => 'W',
                'ʏ' => 'Y',
                'ᴢ' => 'Z',

                // Keep other characters as-is
                c => c,
            })
            .collect()
    }

    /// Check for confusable characters and handle according to security level
    fn check_confusable_identifier(&mut self, identifier: &str, normalized: &str) -> bool {
        if !self.unicode_config.detect_confusables {
            return true; // Confusable detection disabled
        }

        // Fast path for ASCII-only identifiers
        if identifier.is_ascii() {
            return true;
        }

        // Get or compute skeleton using our simple method
        let skeleton = if let Some(cached_skeleton) = self
            .unicode_cache
            .skeleton_cache
            .get(&normalized.to_string())
        {
            cached_skeleton
        } else {
            let skeleton = self.simple_confusable_skeleton(normalized);
            self.unicode_cache
                .skeleton_cache
                .insert(normalized.to_string(), skeleton.clone());
            skeleton
        };

        // Check if this skeleton has been seen before (indicating potential confusables)
        let is_potentially_confusable = self
            .unicode_cache
            .skeleton_cache
            .values()
            .filter(|&s| s == &skeleton)
            .count()
            > 1;

        if is_potentially_confusable {
            let warning_key = format!("{}:{}", skeleton, normalized);

            // Only warn once per confusable pair
            if !self.unicode_cache.warned_confusables.contains(&warning_key) {
                self.unicode_cache.warned_confusables.insert(warning_key);

                match self.unicode_config.level {
                    UnicodeSecurityLevel::Strict => {
                        self.error(&format!(
                            "Confusable identifier '{}' may be visually similar to other identifiers (skeleton: '{}')",
                            normalized, skeleton
                        ));
                        return false;
                    }
                    UnicodeSecurityLevel::Warning => {
                        // For now, we'll use error() but in a full implementation,
                        // this should be a warning that doesn't stop compilation
                        self.error(&format!(
                            "Warning: Identifier '{}' may be confusable with other identifiers (skeleton: '{}')",
                            normalized, skeleton
                        ));
                    }
                    UnicodeSecurityLevel::Permissive => {
                        // Allow without warning
                    }
                }
            }
        }

        true
    }

    fn scan_identifier(&mut self) {
        while self.peek().is_alphanumeric() || self.peek() == '_' {
            self.advance();
        }

        // First extract the identifier into owned string
        let original_string = self.extract_current_lexeme();

        // Special case: standalone underscore is a wildcard token
        if original_string == "_" {
            self.add_token(TokenKind::Underscore);
            return;
        }

        // Process the identifier (normalization and confusable check)
        let final_identifier = self.process_identifier(original_string);

        if let Some(identifier) = final_identifier {
            // Use normalized value for keyword lookup and token creation
            let token_kind = TokenKind::from_keyword(&identifier).unwrap_or_else(|| {
                // Intern the identifier for memory efficiency
                let _interned_index = self.string_interner.intern(identifier.clone());
                TokenKind::Identifier(identifier)
            });
            self.add_token(token_kind);
        }
    }

    /// Extract the current lexeme as an owned String
    fn extract_current_lexeme(&self) -> String {
        if self.start_index <= self.current && self.current <= self.input.len() {
            self.input[self.start_index..self.current].to_string()
        } else {
            String::new() // Safe fallback for invalid indices
        }
    }

    /// Process identifier for normalization and confusable checking
    /// Returns None if the identifier should be rejected
    fn process_identifier(&mut self, original: String) -> Option<String> {
        // Apply Unicode normalization if enabled
        let normalized = if self.unicode_config.normalize_identifiers && !original.is_ascii() {
            self.normalize_identifier(&original)
        } else {
            original.clone()
        };

        // Check for confusable characters
        if !self.check_confusable_identifier(&original, &normalized) {
            // In strict mode, confusable identifiers are rejected
            return None;
        }

        Some(normalized)
    }

    fn advance(&mut self) -> char {
        if self.is_at_end() {
            return '\0'; // Safe default for EOF
        }

        let ch = self.current_char();

        // Advance to next UTF-8 character
        let char_byte_len = ch.len_utf8();
        self.current = match self.current.checked_add(char_byte_len) {
            Some(next) if next <= self.input_bytes.len() => next,
            _ => {
                self.error("Internal error: UTF-8 boundary overflow");
                return '\0';
            }
        };

        self.current_char = match self.current_char.checked_add(1) {
            Some(next) => next,
            None => {
                self.error("Internal error: character index overflow");
                return '\0';
            }
        };

        self.location.advance(ch);
        ch
    }

    fn match_char(&mut self, expected: char) -> bool {
        if self.is_at_end() {
            return false;
        }
        if self.current_char() != expected {
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
            self.current_char()
        }
    }

    fn peek_next(&self) -> char {
        if self.is_at_end() {
            return '\0';
        }

        let current_char_len = self.char_byte_len_at(self.current);
        match self.current.checked_add(current_char_len) {
            Some(next_byte_offset) => self.char_at_byte_offset(next_byte_offset),
            None => '\0',
        }
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.input_bytes.len()
    }

    /// Get the current character at the current byte position
    fn current_char(&self) -> char {
        self.char_at_byte_offset(self.current)
    }

    /// Safely decode UTF-8 character at given byte offset
    fn char_at_byte_offset(&self, byte_offset: usize) -> char {
        if byte_offset >= self.input_bytes.len() {
            return '\0';
        }

        // Use string slice for proper UTF-8 decoding
        match self.input[byte_offset..].chars().next() {
            Some(ch) => ch,
            None => '\0', // Should not happen with valid UTF-8
        }
    }

    /// Get the byte length of the character at the given byte offset
    fn char_byte_len_at(&self, byte_offset: usize) -> usize {
        self.char_at_byte_offset(byte_offset).len_utf8()
    }

    fn add_token(&mut self, kind: TokenKind) {
        // Check token count limit
        if self.tokens.len() >= MAX_TOKEN_COUNT {
            self.error_with_details(
                "Token count exceeds maximum allowed limit",
                &format!(
                    "Token count {} exceeds maximum allowed count of {}",
                    self.tokens.len(),
                    MAX_TOKEN_COUNT
                ),
            );
            return;
        }

        let span = Span::new(self.start_location, self.location);
        let lexeme_slice = if self.start_index <= self.current && self.current <= self.input.len() {
            &self.input[self.start_index..self.current]
        } else {
            "" // Safe fallback for invalid indices
        };

        // Intern lexeme for memory efficiency
        let interned_index = self.string_interner.intern(lexeme_slice.to_string());
        let lexeme = self
            .string_interner
            .get(interned_index)
            .map(|s| s.to_string())
            .unwrap_or_default();
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
            // Check content length limit
            if content.len() >= MAX_STRING_LITERAL_SIZE {
                self.error(&format!(
                    "Doc comment length {} exceeds maximum allowed size of {} bytes",
                    content.len(),
                    MAX_STRING_LITERAL_SIZE
                ));
                return;
            }
            content.push(self.advance());
        }

        let trimmed_content = content.trim();
        let interned_index = self.string_interner.intern(trimmed_content.to_string());
        if let Some(interned_string) = self.string_interner.get(interned_index) {
            self.add_token(TokenKind::DocComment(interned_string.to_string()));
        }
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
            // Check content length limit
            if content.len() >= MAX_STRING_LITERAL_SIZE {
                self.error(&format!(
                    "Doc comment length {} exceeds maximum allowed size of {} bytes",
                    content.len(),
                    MAX_STRING_LITERAL_SIZE
                ));
                return;
            }

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
        let trimmed = content.trim();
        let interned_index = self.string_interner.intern(trimmed.to_string());
        if let Some(interned_string) = self.string_interner.get(interned_index) {
            self.add_token(TokenKind::DocComment(interned_string.to_string()));
        }
    }

    fn error(&mut self, message: &str) {
        self.errors
            .push(Error::lexer(message).with_location(self.location));
    }

    /// Error with optional debug information
    fn error_with_details(&mut self, production_msg: &str, debug_msg: &str) {
        if PRODUCTION_ERRORS {
            self.error(production_msg);
        } else {
            self.error(debug_msg);
        }
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
