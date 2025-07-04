use super::*;
use crate::error::Error;

fn scan(input: &str) -> Vec<TokenKind> {
    let lexer = Lexer::new(input);
    let (tokens, errors) = lexer.scan_tokens();

    assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);

    tokens
        .into_iter()
        .filter(|t| !matches!(t.kind, TokenKind::Newline | TokenKind::Eof))
        .map(|t| t.kind)
        .collect()
}

fn scan_with_errors(input: &str) -> (Vec<TokenKind>, Vec<Error>) {
    let lexer = Lexer::new(input);
    let (tokens, errors) = lexer.scan_tokens();

    let kinds = tokens
        .into_iter()
        .filter(|t| !matches!(t.kind, TokenKind::Newline | TokenKind::Eof))
        .map(|t| t.kind)
        .collect();

    (kinds, errors)
}

#[test]
fn test_single_character_tokens() {
    let input = "( ) { } [ ] , . ; : + - * / % @";
    let tokens = scan(input);

    assert_eq!(
        tokens,
        vec![
            TokenKind::LeftParen,
            TokenKind::RightParen,
            TokenKind::LeftBrace,
            TokenKind::RightBrace,
            TokenKind::LeftBracket,
            TokenKind::RightBracket,
            TokenKind::Comma,
            TokenKind::Dot,
            TokenKind::Semicolon,
            TokenKind::Colon,
            TokenKind::Plus,
            TokenKind::Minus,
            TokenKind::Star,
            TokenKind::Slash,
            TokenKind::Percent,
            TokenKind::At,
        ]
    );
}

#[test]
fn test_two_character_tokens() {
    let input = "== != <= >= && || -> ..";
    let tokens = scan(input);

    assert_eq!(
        tokens,
        vec![
            TokenKind::EqualsEquals,
            TokenKind::BangEquals,
            TokenKind::LessEquals,
            TokenKind::GreaterEquals,
            TokenKind::And,
            TokenKind::Or,
            TokenKind::Arrow,
            TokenKind::DotDot,
        ]
    );
}

#[test]
fn test_comparison_operators() {
    let input = "< > <= >= == !=";
    let tokens = scan(input);

    assert_eq!(
        tokens,
        vec![
            TokenKind::Less,
            TokenKind::Greater,
            TokenKind::LessEquals,
            TokenKind::GreaterEquals,
            TokenKind::EqualsEquals,
            TokenKind::BangEquals,
        ]
    );
}

#[test]
fn test_keywords() {
    let input = "fn let if else while for return true false";
    let tokens = scan(input);

    assert_eq!(
        tokens,
        vec![
            TokenKind::Fn,
            TokenKind::Let,
            TokenKind::If,
            TokenKind::Else,
            TokenKind::While,
            TokenKind::For,
            TokenKind::Return,
            TokenKind::True,
            TokenKind::False,
        ]
    );
}

#[test]
fn test_module_keywords() {
    let input = "import export from as in";
    let tokens = scan(input);

    assert_eq!(
        tokens,
        vec![
            TokenKind::Import,
            TokenKind::Export,
            TokenKind::From,
            TokenKind::As,
            TokenKind::In,
        ]
    );
}

#[test]
fn test_identifiers() {
    let input = "foo bar_baz _private camelCase PascalCase number123";
    let tokens = scan(input);

    assert_eq!(
        tokens,
        vec![
            TokenKind::Identifier("foo".to_string()),
            TokenKind::Identifier("bar_baz".to_string()),
            TokenKind::Identifier("_private".to_string()),
            TokenKind::Identifier("camelCase".to_string()),
            TokenKind::Identifier("PascalCase".to_string()),
            TokenKind::Identifier("number123".to_string()),
        ]
    );
}

#[test]
fn test_numbers() {
    let input = "42 3.14 0 0.0 100.001";
    let tokens = scan(input);

    assert_eq!(
        tokens,
        vec![
            TokenKind::Number(42.0),
            TokenKind::Number(3.14),
            TokenKind::Number(0.0),
            TokenKind::Number(0.0),
            TokenKind::Number(100.001),
        ]
    );
}

#[test]
fn test_strings() {
    let input = r#""hello" "world" "Script lang" "with\nnewline" "with\ttab" "quote\"inside""#;
    let tokens = scan(input);

    assert_eq!(
        tokens,
        vec![
            TokenKind::String("hello".to_string()),
            TokenKind::String("world".to_string()),
            TokenKind::String("Script lang".to_string()),
            TokenKind::String("with\nnewline".to_string()),
            TokenKind::String("with\ttab".to_string()),
            TokenKind::String("quote\"inside".to_string()),
        ]
    );
}

#[test]
fn test_single_line_comments() {
    let input = "// This is a comment\nlet x = 42 // Another comment";
    let tokens = scan(input);

    assert_eq!(
        tokens,
        vec![
            TokenKind::Let,
            TokenKind::Identifier("x".to_string()),
            TokenKind::Equals,
            TokenKind::Number(42.0),
        ]
    );
}

#[test]
fn test_multi_line_comments() {
    let input = "/* This is a\nmulti-line comment */\nlet /* inline */ x = /* another */ 42";
    let tokens = scan(input);

    assert_eq!(
        tokens,
        vec![
            TokenKind::Let,
            TokenKind::Identifier("x".to_string()),
            TokenKind::Equals,
            TokenKind::Number(42.0),
        ]
    );
}

#[test]
fn test_nested_comments() {
    let input = "/* outer /* inner */ still in comment */ let x = 42";
    let tokens = scan(input);

    assert_eq!(
        tokens,
        vec![
            TokenKind::Let,
            TokenKind::Identifier("x".to_string()),
            TokenKind::Equals,
            TokenKind::Number(42.0),
        ]
    );
}

#[test]
fn test_arithmetic_expression() {
    let input = "2 + 3 * 4 - 5 / 2";
    let tokens = scan(input);

    assert_eq!(
        tokens,
        vec![
            TokenKind::Number(2.0),
            TokenKind::Plus,
            TokenKind::Number(3.0),
            TokenKind::Star,
            TokenKind::Number(4.0),
            TokenKind::Minus,
            TokenKind::Number(5.0),
            TokenKind::Slash,
            TokenKind::Number(2.0),
        ]
    );
}

#[test]
fn test_function_declaration() {
    let input = "fn add(a: i32, b: i32) -> i32 { return a + b }";
    let tokens = scan(input);

    assert_eq!(
        tokens,
        vec![
            TokenKind::Fn,
            TokenKind::Identifier("add".to_string()),
            TokenKind::LeftParen,
            TokenKind::Identifier("a".to_string()),
            TokenKind::Colon,
            TokenKind::Identifier("i32".to_string()),
            TokenKind::Comma,
            TokenKind::Identifier("b".to_string()),
            TokenKind::Colon,
            TokenKind::Identifier("i32".to_string()),
            TokenKind::RightParen,
            TokenKind::Arrow,
            TokenKind::Identifier("i32".to_string()),
            TokenKind::LeftBrace,
            TokenKind::Return,
            TokenKind::Identifier("a".to_string()),
            TokenKind::Plus,
            TokenKind::Identifier("b".to_string()),
            TokenKind::RightBrace,
        ]
    );
}

#[test]
fn test_unterminated_string() {
    let input = r#""unterminated"#;
    let (tokens, errors) = scan_with_errors(input);

    assert!(tokens.is_empty());
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("Unterminated string"));
}

#[test]
fn test_invalid_escape_sequence() {
    let input = r#""invalid\x""#;
    let (tokens, errors) = scan_with_errors(input);

    assert_eq!(tokens.len(), 1);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("Invalid escape sequence"));
}

#[test]
fn test_unexpected_character() {
    // Use a character that is truly unexpected (not part of the language)
    let input = "let x = 42 $ error";
    let (_tokens, errors) = scan_with_errors(input);

    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("Unexpected character: '$'"));
}

#[test]
fn test_whitespace_handling() {
    let input = "  let   x\t=\n42  ";
    let tokens = scan(input);

    assert_eq!(
        tokens,
        vec![
            TokenKind::Let,
            TokenKind::Identifier("x".to_string()),
            TokenKind::Equals,
            TokenKind::Number(42.0),
        ]
    );
}

#[test]
fn test_empty_input() {
    let lexer = Lexer::new("");
    let (tokens, errors) = lexer.scan_tokens();

    assert!(errors.is_empty());
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].kind, TokenKind::Eof);
}

#[test]
fn test_source_location_tracking() {
    let input = "let x = 42\nlet y = 3.14";
    let lexer = Lexer::new(input);
    let (tokens, _) = lexer.scan_tokens();

    // Check first line tokens
    assert_eq!(tokens[0].span.start.line, 1);
    assert_eq!(tokens[0].span.start.column, 1);

    // Check second line tokens
    let second_line_start = tokens.iter().position(|t| t.span.start.line == 2).unwrap();

    assert_eq!(tokens[second_line_start].lexeme, "let");
    assert_eq!(tokens[second_line_start].span.start.column, 1);
}

#[test]
fn test_named_import_statement() {
    let input = r#"import { func, another_func } from "./module""#;
    let tokens = scan(input);

    assert_eq!(
        tokens,
        vec![
            TokenKind::Import,
            TokenKind::LeftBrace,
            TokenKind::Identifier("func".to_string()),
            TokenKind::Comma,
            TokenKind::Identifier("another_func".to_string()),
            TokenKind::RightBrace,
            TokenKind::From,
            TokenKind::String("./module".to_string()),
        ]
    );
}

#[test]
fn test_default_import_statement() {
    let input = r#"import defaultExport from "./module""#;
    let tokens = scan(input);

    assert_eq!(
        tokens,
        vec![
            TokenKind::Import,
            TokenKind::Identifier("defaultExport".to_string()),
            TokenKind::From,
            TokenKind::String("./module".to_string()),
        ]
    );
}

#[test]
fn test_namespace_import_statement() {
    let input = r#"import * as mod from "./lib""#;
    let tokens = scan(input);

    assert_eq!(
        tokens,
        vec![
            TokenKind::Import,
            TokenKind::Star,
            TokenKind::As,
            TokenKind::Identifier("mod".to_string()),
            TokenKind::From,
            TokenKind::String("./lib".to_string()),
        ]
    );
}

#[test]
fn test_aliased_import_statement() {
    let input = r#"import { original as alias, another } from "./module""#;
    let tokens = scan(input);

    assert_eq!(
        tokens,
        vec![
            TokenKind::Import,
            TokenKind::LeftBrace,
            TokenKind::Identifier("original".to_string()),
            TokenKind::As,
            TokenKind::Identifier("alias".to_string()),
            TokenKind::Comma,
            TokenKind::Identifier("another".to_string()),
            TokenKind::RightBrace,
            TokenKind::From,
            TokenKind::String("./module".to_string()),
        ]
    );
}

#[test]
fn test_export_function_declaration() {
    let input = "export fn my_function() { return 42 }";
    let tokens = scan(input);

    assert_eq!(
        tokens,
        vec![
            TokenKind::Export,
            TokenKind::Fn,
            TokenKind::Identifier("my_function".to_string()),
            TokenKind::LeftParen,
            TokenKind::RightParen,
            TokenKind::LeftBrace,
            TokenKind::Return,
            TokenKind::Number(42.0),
            TokenKind::RightBrace,
        ]
    );
}

#[test]
fn test_export_variable_declaration() {
    let input = "export let config = { debug: true }";
    let tokens = scan(input);

    assert_eq!(
        tokens,
        vec![
            TokenKind::Export,
            TokenKind::Let,
            TokenKind::Identifier("config".to_string()),
            TokenKind::Equals,
            TokenKind::LeftBrace,
            TokenKind::Identifier("debug".to_string()),
            TokenKind::Colon,
            TokenKind::True,
            TokenKind::RightBrace,
        ]
    );
}

#[test]
fn test_export_named_declaration() {
    let input = "export { func, variable as alias }";
    let tokens = scan(input);

    assert_eq!(
        tokens,
        vec![
            TokenKind::Export,
            TokenKind::LeftBrace,
            TokenKind::Identifier("func".to_string()),
            TokenKind::Comma,
            TokenKind::Identifier("variable".to_string()),
            TokenKind::As,
            TokenKind::Identifier("alias".to_string()),
            TokenKind::RightBrace,
        ]
    );
}

#[test]
fn test_re_export_statement() {
    let input = r#"export { func } from "./other_module""#;
    let tokens = scan(input);

    assert_eq!(
        tokens,
        vec![
            TokenKind::Export,
            TokenKind::LeftBrace,
            TokenKind::Identifier("func".to_string()),
            TokenKind::RightBrace,
            TokenKind::From,
            TokenKind::String("./other_module".to_string()),
        ]
    );
}

#[test]
fn test_export_all_re_export() {
    let input = r#"export * from "./other_module""#;
    let tokens = scan(input);

    assert_eq!(
        tokens,
        vec![
            TokenKind::Export,
            TokenKind::Star,
            TokenKind::From,
            TokenKind::String("./other_module".to_string()),
        ]
    );
}

#[test]
fn test_complex_module_example() {
    let input = r#"
import { helper, utils as u } from "./helpers"
import * as math from "./math"

export fn process_data(data) {
    return math.calculate(u.transform(data))
}

export { helper }
"#;
    let tokens = scan(input);

    assert_eq!(
        tokens,
        vec![
            TokenKind::Import,
            TokenKind::LeftBrace,
            TokenKind::Identifier("helper".to_string()),
            TokenKind::Comma,
            TokenKind::Identifier("utils".to_string()),
            TokenKind::As,
            TokenKind::Identifier("u".to_string()),
            TokenKind::RightBrace,
            TokenKind::From,
            TokenKind::String("./helpers".to_string()),
            TokenKind::Import,
            TokenKind::Star,
            TokenKind::As,
            TokenKind::Identifier("math".to_string()),
            TokenKind::From,
            TokenKind::String("./math".to_string()),
            TokenKind::Export,
            TokenKind::Fn,
            TokenKind::Identifier("process_data".to_string()),
            TokenKind::LeftParen,
            TokenKind::Identifier("data".to_string()),
            TokenKind::RightParen,
            TokenKind::LeftBrace,
            TokenKind::Return,
            TokenKind::Identifier("math".to_string()),
            TokenKind::Dot,
            TokenKind::Identifier("calculate".to_string()),
            TokenKind::LeftParen,
            TokenKind::Identifier("u".to_string()),
            TokenKind::Dot,
            TokenKind::Identifier("transform".to_string()),
            TokenKind::LeftParen,
            TokenKind::Identifier("data".to_string()),
            TokenKind::RightParen,
            TokenKind::RightParen,
            TokenKind::RightBrace,
            TokenKind::Export,
            TokenKind::LeftBrace,
            TokenKind::Identifier("helper".to_string()),
            TokenKind::RightBrace,
        ]
    );
}

#[test]
fn test_module_keywords_not_in_string() {
    let input = r#""import export from as" import real_keyword"#;
    let tokens = scan(input);

    assert_eq!(
        tokens,
        vec![
            TokenKind::String("import export from as".to_string()),
            TokenKind::Import,
            TokenKind::Identifier("real_keyword".to_string()),
        ]
    );
}

#[test]
fn test_module_keywords_in_identifiers() {
    let input = "importData exportHelper fromStart asValue";
    let tokens = scan(input);

    assert_eq!(
        tokens,
        vec![
            TokenKind::Identifier("importData".to_string()),
            TokenKind::Identifier("exportHelper".to_string()),
            TokenKind::Identifier("fromStart".to_string()),
            TokenKind::Identifier("asValue".to_string()),
        ]
    );
}

#[test]
fn test_single_line_doc_comment() {
    let input = "/// This is a doc comment\nfn test() {}";
    let tokens = scan(input);

    assert_eq!(
        tokens,
        vec![
            TokenKind::DocComment("This is a doc comment".to_string()),
            TokenKind::Fn,
            TokenKind::Identifier("test".to_string()),
            TokenKind::LeftParen,
            TokenKind::RightParen,
            TokenKind::LeftBrace,
            TokenKind::RightBrace,
        ]
    );
}

#[test]
fn test_multiple_doc_comments() {
    let input = r#"
/// First line of docs
/// Second line of docs
fn example() {}
"#;
    let tokens = scan(input);

    assert_eq!(
        tokens,
        vec![
            TokenKind::DocComment("First line of docs".to_string()),
            TokenKind::DocComment("Second line of docs".to_string()),
            TokenKind::Fn,
            TokenKind::Identifier("example".to_string()),
            TokenKind::LeftParen,
            TokenKind::RightParen,
            TokenKind::LeftBrace,
            TokenKind::RightBrace,
        ]
    );
}

#[test]
fn test_multiline_doc_comment() {
    let input = r#"
/**
 * This is a multiline doc comment
 * with multiple lines
 * of documentation
 */
fn test() {}
"#;
    let tokens = scan(input);

    assert_eq!(
        tokens,
        vec![
            TokenKind::DocComment(
                "This is a multiline doc comment\nwith multiple lines\nof documentation"
                    .to_string()
            ),
            TokenKind::Fn,
            TokenKind::Identifier("test".to_string()),
            TokenKind::LeftParen,
            TokenKind::RightParen,
            TokenKind::LeftBrace,
            TokenKind::RightBrace,
        ]
    );
}

#[test]
fn test_doc_comment_without_space() {
    let input = "///No space after slashes\nfn test() {}";
    let tokens = scan(input);

    assert_eq!(
        tokens,
        vec![
            TokenKind::DocComment("No space after slashes".to_string()),
            TokenKind::Fn,
            TokenKind::Identifier("test".to_string()),
            TokenKind::LeftParen,
            TokenKind::RightParen,
            TokenKind::LeftBrace,
            TokenKind::RightBrace,
        ]
    );
}

#[test]
fn test_regular_comments_not_doc() {
    let input = r#"
// Regular comment
/* Regular multiline */
fn test() {}
"#;
    let tokens = scan(input);

    // Regular comments are not tokenized
    assert_eq!(
        tokens,
        vec![
            TokenKind::Fn,
            TokenKind::Identifier("test".to_string()),
            TokenKind::LeftParen,
            TokenKind::RightParen,
            TokenKind::LeftBrace,
            TokenKind::RightBrace,
        ]
    );
}
#[test]
fn test_struct_enum_where_keywords() {
    let input = "struct enum where";
    let tokens = scan(input);

    assert_eq!(
        tokens,
        vec![
            TokenKind::Struct,
            TokenKind::Enum,
            TokenKind::Where,
        ]
    );
}

#[test]
fn test_colon_colon_operator() {
    let input = "Status::Ok Vec::new";
    let tokens = scan(input);

    assert_eq!(
        tokens,
        vec![
            TokenKind::Identifier("Status".to_string()),
            TokenKind::ColonColon,
            TokenKind::Identifier("Ok".to_string()),
            TokenKind::Identifier("Vec".to_string()),
            TokenKind::ColonColon,
            TokenKind::Identifier("new".to_string()),
        ]
    );
}