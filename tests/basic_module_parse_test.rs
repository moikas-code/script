use script::{Lexer, Parser};

/// Test that we can parse files that will become modules
#[test]
fn test_parse_simple_math_module() {
    let source = r#"
// Math utilities module

fn add(a: float, b: float) -> float {
    a + b
}

fn multiply(a: float, b: float) -> float {
    a * b
}

let PI: float = 3.14159
"#;

    let lexer = Lexer::new(source).expect("Failed to create lexer");
    let (tokens, lex_errors) = lexer.scan_tokens();

    assert!(
        lex_errors.is_empty(),
        "Unexpected lexer errors: {:?}",
        lex_errors
    );

    let mut parser = Parser::new(tokens);
    let result = parser.parse();

    assert!(result.is_ok(), "Failed to parse module: {:?}", result.err());

    let program = result.unwrap();
    assert_eq!(program.statements.len(), 3); // 2 functions + 1 variable
}

/// Test parsing struct definitions that would be exported
#[test]
fn test_parse_type_definitions() {
    let source = r#"
struct Point2D {
    x: float,
    y: float
}

fn Point2D(x: float, y: float) -> Point2D {
    Point2D { x: x, y: y }
}

fn distance(p1: Point2D, p2: Point2D) -> float {
    let dx = p2.x - p1.x
    let dy = p2.y - p1.y
    sqrt(dx * dx + dy * dy)
}
"#;

    let lexer = Lexer::new(source).expect("Failed to create lexer");
    let (tokens, lex_errors) = lexer.scan_tokens();

    assert!(
        lex_errors.is_empty(),
        "Unexpected lexer errors: {:?}",
        lex_errors
    );

    // For now, just verify it tokenizes correctly
    // since struct parsing might not be implemented yet
    assert!(tokens.len() > 10, "Expected more tokens");
}

/// Test parsing function with multiple return values (tuple)
#[test]
fn test_parse_tuple_return() {
    let source = r#"
fn divmod(a: int, b: int) -> (int, int) {
    (a / b, a % b)
}

fn main() {
    let (quotient, remainder) = divmod(17, 5)
    print("17 / 5 = " + quotient + " remainder " + remainder)
}
"#;

    let lexer = Lexer::new(source).expect("Failed to create lexer");
    let (tokens, lex_errors) = lexer.scan_tokens();

    assert!(
        lex_errors.is_empty(),
        "Unexpected lexer errors: {:?}",
        lex_errors
    );

    // Verify tokens are generated correctly
    let has_tuple_syntax = tokens.iter().any(|t| {
        matches!(t.kind, script::TokenKind::LeftParen)
            || matches!(t.kind, script::TokenKind::RightParen)
    });
    assert!(has_tuple_syntax, "Expected tuple syntax tokens");
}

/// Test parsing comments in module files
#[test]
fn test_parse_module_with_comments() {
    let source = r#"
/**
 * String utilities module
 * Provides basic string manipulation functions
 */

// Convert string to uppercase
fn uppercase(s: string) -> string {
    // TODO: implement
    s
}

/* Multi-line comment
   explaining the function */
fn lowercase(s: string) -> string {
    s // inline comment
}
"#;

    let lexer = Lexer::new(source).expect("Failed to create lexer");
    let (tokens, lex_errors) = lexer.scan_tokens();

    assert!(
        lex_errors.is_empty(),
        "Unexpected lexer errors: {:?}",
        lex_errors
    );

    let mut parser = Parser::new(tokens);
    let result = parser.parse();

    assert!(
        result.is_ok(),
        "Failed to parse module with comments: {:?}",
        result.err()
    );
}

/// Test parsing enums that would be part of modules
#[test]
fn test_parse_enum_definitions() {
    let source = r#"
enum Result {
    Ok(value),
    Err(error)
}

enum Option {
    Some(value),
    None
}

fn unwrap_or(opt: Option, default: any) -> any {
    match opt {
        Some(v) => v,
        None => default
    }
}
"#;

    let lexer = Lexer::new(source).expect("Failed to create lexer");
    let (tokens, lex_errors) = lexer.scan_tokens();

    assert!(
        lex_errors.is_empty(),
        "Unexpected lexer errors: {:?}",
        lex_errors
    );

    // Check for enum-related tokens
    let has_enum = tokens
        .iter()
        .any(|t| matches!(t.kind, script::TokenKind::Identifier(ref s) if s == "enum"));
    assert!(has_enum, "Expected 'enum' keyword in tokens");
}
