use crate::lexer::{Lexer, TokenKind};
use crate::lsp::capabilities::SUPPORTED_TOKEN_TYPES;
use tower_lsp::lsp_types::{SemanticToken, SemanticTokenType};

/// Convert a Script token to an LSP semantic token type index
fn token_kind_to_semantic_type(kind: &TokenKind) -> Option<u32> {
    let token_type = match kind {
        // Keywords
        TokenKind::Let
        | TokenKind::Fn
        | TokenKind::Return
        | TokenKind::If
        | TokenKind::Else
        | TokenKind::While
        | TokenKind::For
        | TokenKind::In
        | TokenKind::True
        | TokenKind::False
        | TokenKind::Import
        | TokenKind::Export
        | TokenKind::As
        | TokenKind::From
        | TokenKind::Async
        | TokenKind::Await
        | TokenKind::Match
        | TokenKind::Print => SemanticTokenType::KEYWORD,

        // Identifiers (could be variables, functions, types, etc.)
        TokenKind::Identifier(_) => SemanticTokenType::VARIABLE,

        // Literals
        TokenKind::Number(_) => SemanticTokenType::NUMBER,
        TokenKind::String(_) => SemanticTokenType::STRING,

        // Operators
        TokenKind::Plus
        | TokenKind::Minus
        | TokenKind::Star
        | TokenKind::Slash
        | TokenKind::Percent
        | TokenKind::Equals
        | TokenKind::EqualsEquals
        | TokenKind::BangEquals
        | TokenKind::Less
        | TokenKind::Greater
        | TokenKind::LessEquals
        | TokenKind::GreaterEquals
        | TokenKind::And
        | TokenKind::Or
        | TokenKind::Bang
        | TokenKind::Arrow
        | TokenKind::DoubleArrow
        | TokenKind::At
        | TokenKind::DotDot => SemanticTokenType::OPERATOR,

        // Skip punctuation and other tokens
        _ => return None,
    };

    // Find the index of this token type in our supported list
    SUPPORTED_TOKEN_TYPES
        .iter()
        .position(|t| *t == token_type)
        .map(|pos| pos as u32)
}

/// Generate semantic tokens for a Script source file
pub fn generate_semantic_tokens(source: &str) -> Vec<SemanticToken> {
    let mut lexer = match Lexer::new(source) {
        Ok(lexer) => lexer,
        Err(_) => return Vec::new(), // Return empty tokens on lexer initialization error
    };
    let mut tokens = Vec::new();
    let mut prev_line = 0;
    let mut prev_col = 0;

    loop {
        let (token_opt, _errors) = lexer.next_token();
        let token = match token_opt {
            Some(t) => t,
            None => break,
        };

        if matches!(token.kind, TokenKind::Eof) {
            break;
        }

        // Skip tokens we don't want to highlight
        if let Some(token_type) = token_kind_to_semantic_type(&token.kind) {
            let line = token.span.start.line as u32;
            let col = token.span.start.column as u32;
            let length = (token.span.end.byte_offset - token.span.start.byte_offset) as u32;

            // LSP semantic tokens use delta encoding
            let delta_line = line - prev_line;
            let delta_start = if delta_line == 0 { col - prev_col } else { col };

            tokens.push(SemanticToken {
                delta_line,
                delta_start,
                length,
                token_type,
                token_modifiers_bitset: 0, // We'll add modifiers later
            });

            prev_line = line;
            prev_col = col;
        }
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semantic_tokens_basic() {
        let source = r#"
let x = 42;
fn add(a, b) {
    return a + b;
}
"#;

        let tokens = generate_semantic_tokens(source);
        assert!(!tokens.is_empty());

        // Verify first token is 'let' keyword
        let first = &tokens[0];
        assert_eq!(first.length, 3); // "let" is 3 characters
        assert_eq!(
            first.token_type,
            SUPPORTED_TOKEN_TYPES
                .iter()
                .position(|t| *t == SemanticTokenType::KEYWORD)
                .unwrap() as u32
        );
    }

    #[test]
    fn test_semantic_tokens_strings() {
        let source = r#"
let name = "Script";
let greeting = "Hello, World!";
"#;

        let tokens = generate_semantic_tokens(source);

        // Should have tokens for let, name, =, string, let, greeting, =, string
        // But semantic tokens may not include all tokens (e.g., operators)
        assert!(
            !tokens.is_empty(),
            "Expected some semantic tokens but got none"
        );
    }
}
