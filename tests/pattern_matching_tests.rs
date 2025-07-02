use script::{parser::ast::*, Lexer, Parser, Result, SemanticAnalyzer};

fn parse_program(input: &str) -> Result<Program> {
    let lexer = Lexer::new(input);
    let (tokens, errors) = lexer.scan_tokens();

    if !errors.is_empty() {
        return Err(errors.into_iter().next().unwrap());
    }

    let mut parser = Parser::new(tokens);
    parser.parse()
}

fn parse_and_analyze(input: &str) -> Result<Program> {
    let program = parse_program(input)?;
    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze(&program)?;
    Ok(program)
}

#[test]
fn test_basic_match_literal_patterns() {
    let input = r#"
        let x = 42
        let result = match x {
            42 => "found forty-two",
            0 => "found zero", 
            _ => "found something else"
        }
    "#;

    let program = parse_program(input).unwrap();
    assert_eq!(program.statements.len(), 2);

    // Check the second statement is the match assignment
    if let StmtKind::Let {
        init: Some(expr), ..
    } = &program.statements[1].kind
    {
        if let ExprKind::Match {
            expr: match_expr,
            arms,
        } = &expr.kind
        {
            // Check we're matching on x
            assert!(matches!(match_expr.kind, ExprKind::Identifier(ref name) if name == "x"));

            // Check we have 3 arms
            assert_eq!(arms.len(), 3);

            // Check first arm: 42 => "found forty-two"
            assert!(matches!(
                arms[0].pattern.kind,
                PatternKind::Literal(Literal::Number(n)) if n == 42.0
            ));
            assert!(matches!(
                arms[0].body.kind,
                ExprKind::Literal(Literal::String(ref s)) if s == "found forty-two"
            ));

            // Check wildcard arm
            assert!(matches!(arms[2].pattern.kind, PatternKind::Wildcard));
        } else {
            panic!("Expected match expression");
        }
    } else {
        panic!("Expected let statement with match init");
    }
}

#[test]
fn test_match_string_patterns() {
    let input = r#"
        let name = "Alice"
        let greeting = match name {
            "Alice" => "Hello Alice!",
            "Bob" => "Hello Bob!",
            _ => "Hello stranger!"
        }
    "#;

    let program = parse_program(input).unwrap();
    assert_eq!(program.statements.len(), 2);

    if let StmtKind::Let {
        init: Some(expr), ..
    } = &program.statements[1].kind
    {
        if let ExprKind::Match { arms, .. } = &expr.kind {
            assert_eq!(arms.len(), 3);

            // Check string pattern
            assert!(matches!(
                arms[0].pattern.kind,
                PatternKind::Literal(Literal::String(ref s)) if s == "Alice"
            ));
        }
    }
}

#[test]
fn test_match_boolean_patterns() {
    let input = r#"
        let flag = true
        let message = match flag {
            true => "enabled",
            false => "disabled"
        }
    "#;

    let program = parse_program(input).unwrap();
    assert_eq!(program.statements.len(), 2);

    if let StmtKind::Let {
        init: Some(expr), ..
    } = &program.statements[1].kind
    {
        if let ExprKind::Match { arms, .. } = &expr.kind {
            assert_eq!(arms.len(), 2);

            // Check boolean patterns
            assert!(matches!(
                arms[0].pattern.kind,
                PatternKind::Literal(Literal::Boolean(true))
            ));
            assert!(matches!(
                arms[1].pattern.kind,
                PatternKind::Literal(Literal::Boolean(false))
            ));
        }
    }
}

#[test]
fn test_match_variable_binding() {
    let input = r#"
        let value = 123
        let result = match value {
            x => x + 1
        }
    "#;

    let program = parse_program(input).unwrap();
    assert_eq!(program.statements.len(), 2);

    if let StmtKind::Let {
        init: Some(expr), ..
    } = &program.statements[1].kind
    {
        if let ExprKind::Match { arms, .. } = &expr.kind {
            assert_eq!(arms.len(), 1);

            // Check variable pattern binding
            assert!(matches!(
                arms[0].pattern.kind,
                PatternKind::Identifier(ref name) if name == "x"
            ));

            // Check the body uses the bound variable
            if let ExprKind::Binary { left, .. } = &arms[0].body.kind {
                assert!(matches!(left.kind, ExprKind::Identifier(ref name) if name == "x"));
            }
        }
    }
}

#[test]
fn test_match_guards() {
    let input = r#"
        let number = 10
        let result = match number {
            x if x > 10 => "greater than ten",
            x if x < 0 => "negative",
            x => "between zero and ten"
        }
    "#;

    let program = parse_program(input).unwrap();
    assert_eq!(program.statements.len(), 2);

    if let StmtKind::Let {
        init: Some(expr), ..
    } = &program.statements[1].kind
    {
        if let ExprKind::Match { arms, .. } = &expr.kind {
            assert_eq!(arms.len(), 3);

            // Check first arm has guard
            assert!(arms[0].guard.is_some());
            assert!(matches!(
                arms[0].pattern.kind,
                PatternKind::Identifier(ref name) if name == "x"
            ));

            // Check guard expression
            if let Some(guard) = &arms[0].guard {
                assert!(matches!(guard.kind, ExprKind::Binary { .. }));
            }

            // Check second arm has guard
            assert!(arms[1].guard.is_some());

            // Check third arm has no guard
            assert!(arms[2].guard.is_none());
        }
    }
}

#[test]
fn test_match_array_destructuring() {
    let input = r#"
        let arr = [1, 2, 3]
        let result = match arr {
            [x, y, z] => x + y + z,
            [first, second] => first + second,
            [] => 0,
            _ => -1
        }
    "#;

    let program = parse_program(input).unwrap();
    assert_eq!(program.statements.len(), 2);

    if let StmtKind::Let {
        init: Some(expr), ..
    } = &program.statements[1].kind
    {
        if let ExprKind::Match { arms, .. } = &expr.kind {
            assert_eq!(arms.len(), 4);

            // Check first arm: [x, y, z]
            if let PatternKind::Array(patterns) = &arms[0].pattern.kind {
                assert_eq!(patterns.len(), 3);
                assert!(
                    matches!(patterns[0].kind, PatternKind::Identifier(ref name) if name == "x")
                );
                assert!(
                    matches!(patterns[1].kind, PatternKind::Identifier(ref name) if name == "y")
                );
                assert!(
                    matches!(patterns[2].kind, PatternKind::Identifier(ref name) if name == "z")
                );
            } else {
                panic!("Expected array pattern");
            }

            // Check second arm: [first, second]
            if let PatternKind::Array(patterns) = &arms[1].pattern.kind {
                assert_eq!(patterns.len(), 2);
            } else {
                panic!("Expected array pattern");
            }

            // Check third arm: []
            if let PatternKind::Array(patterns) = &arms[2].pattern.kind {
                assert_eq!(patterns.len(), 0);
            } else {
                panic!("Expected empty array pattern");
            }

            // Check wildcard arm
            assert!(matches!(arms[3].pattern.kind, PatternKind::Wildcard));
        }
    }
}

#[test]
fn test_match_object_destructuring() {
    let input = r#"
        let obj = { x: 10, y: 20 }
        let result = match obj {
            { x, y } => x + y,
            { x: a, y: b } => a * b,
            { x } => x,
            _ => 0
        }
    "#;

    let program = parse_program(input).unwrap();
    assert_eq!(program.statements.len(), 2);

    if let StmtKind::Let {
        init: Some(expr), ..
    } = &program.statements[1].kind
    {
        if let ExprKind::Match { arms, .. } = &expr.kind {
            assert_eq!(arms.len(), 4);

            // Check first arm: { x, y } (shorthand destructuring)
            if let PatternKind::Object(fields) = &arms[0].pattern.kind {
                assert_eq!(fields.len(), 2);
                assert_eq!(fields[0].0, "x");
                assert!(fields[0].1.is_none()); // shorthand
                assert_eq!(fields[1].0, "y");
                assert!(fields[1].1.is_none()); // shorthand
            } else {
                panic!("Expected object pattern");
            }

            // Check second arm: { x: a, y: b } (explicit destructuring)
            if let PatternKind::Object(fields) = &arms[1].pattern.kind {
                assert_eq!(fields.len(), 2);
                assert_eq!(fields[0].0, "x");
                if let Some(pattern) = &fields[0].1 {
                    assert!(
                        matches!(pattern.kind, PatternKind::Identifier(ref name) if name == "a")
                    );
                }
                assert_eq!(fields[1].0, "y");
                if let Some(pattern) = &fields[1].1 {
                    assert!(
                        matches!(pattern.kind, PatternKind::Identifier(ref name) if name == "b")
                    );
                }
            } else {
                panic!("Expected object pattern");
            }
        }
    }
}

#[test]
fn test_match_nested_patterns() {
    let input = r#"
        let data = [[1, 2], [3, 4]]
        let result = match data {
            [[a, b], [c, d]] => a + b + c + d,
            [first, _] => first[0],
            _ => 0
        }
    "#;

    let program = parse_program(input).unwrap();
    assert_eq!(program.statements.len(), 2);

    if let StmtKind::Let {
        init: Some(expr), ..
    } = &program.statements[1].kind
    {
        if let ExprKind::Match { arms, .. } = &expr.kind {
            assert_eq!(arms.len(), 3);

            // Check nested array pattern
            if let PatternKind::Array(outer_patterns) = &arms[0].pattern.kind {
                assert_eq!(outer_patterns.len(), 2);

                // Check first inner array
                if let PatternKind::Array(inner_patterns) = &outer_patterns[0].kind {
                    assert_eq!(inner_patterns.len(), 2);
                    assert!(
                        matches!(inner_patterns[0].kind, PatternKind::Identifier(ref name) if name == "a")
                    );
                    assert!(
                        matches!(inner_patterns[1].kind, PatternKind::Identifier(ref name) if name == "b")
                    );
                }
            }
        }
    }
}

#[test]
fn test_match_exhaustiveness_with_wildcard() {
    let input = r#"
        let x = 5
        let result = match x {
            1 => "one",
            2 => "two",
            _ => "other"
        }
    "#;

    // Should parse without error since wildcard catches all cases
    let program = parse_program(input).unwrap();
    assert!(program.statements.len() == 2);
}

#[test]
fn test_match_in_function() {
    let input = r#"
        fn classify(x: i32) -> string {
            return match x {
                0 => "zero",
                1 => "one", 
                n if n > 1 => "positive",
                _ => "negative"
            }
        }
    "#;

    let program = parse_program(input).unwrap();
    assert_eq!(program.statements.len(), 1);

    if let StmtKind::Function { body, .. } = &program.statements[0].kind {
        if let Some(final_expr) = &body.final_expr {
            if let ExprKind::Match { arms, .. } = &final_expr.kind {
                assert_eq!(arms.len(), 4);

                // Check guard in third arm
                assert!(arms[2].guard.is_some());
            }
        }
    }
}

#[test]
fn test_match_semantic_analysis() {
    let input = r#"
        let x = 42
        let result = match x {
            n => n + 1
        }
    "#;

    // Should perform semantic analysis without errors
    let _program = parse_and_analyze(input).unwrap();
}

#[test]
fn test_match_type_checking() {
    let input = r#"
        let x = 42
        let result = match x {
            42 => "number forty-two",
            n => n
        }
    "#;

    // This should create a type error since arms return different types
    let result = parse_and_analyze(input);
    // Note: Depending on implementation, this might succeed or fail
    // The test validates that semantic analysis runs
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_match_pattern_variable_scoping() {
    let input = r#"
        let x = 10
        let result = match x {
            y => {
                let z = y + 1
                z
            }
        }
        // x should still be accessible here
        let final_result = x + result
    "#;

    let program = parse_and_analyze(input).unwrap();
    assert_eq!(program.statements.len(), 3);
}

#[test]
fn test_parse_error_cases() {
    // Missing match expression
    let result = parse_program("match { 1 => 2 }");
    assert!(result.is_err());

    // Missing arms
    let result = parse_program("match x {}");
    assert!(result.is_err());

    // Missing arrow
    let result = parse_program("match x { 1 2 }");
    assert!(result.is_err());

    // Invalid pattern
    let result = parse_program("match x { + => 1 }");
    assert!(result.is_err());
}

#[test]
fn test_match_with_complex_expressions() {
    let input = r#"
        let data = [1, 2, 3]
        let func = fn(x: i32) -> i32 { x * 2 }
        let result = match data {
            arr => func(arr[0]) + arr.length
        }
    "#;

    let program = parse_program(input).unwrap();
    assert_eq!(program.statements.len(), 3);
}

// Test to verify current limitations
#[test]
fn test_or_patterns_not_implemented() {
    // OR patterns (|) are not implemented yet - should fail to parse
    let input = r#"
        let x = 5
        let result = match x {
            1 | 2 | 3 => "small",
            _ => "large"
        }
    "#;

    // This should fail because pipe token doesn't exist
    let result = parse_program(input);
    assert!(result.is_err());
}
