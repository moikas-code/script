use script::{Lexer, Parser};
use std::fs;
use std::path::PathBuf;

/// Test that all module examples can be parsed successfully
#[test]
fn test_calculator_example_parses() {
    let calc_dir = PathBuf::from("examples/modules/calculator");
    let files = [
        "main.script",
        "operations.script",
        "types.script",
        "parser.script",
        "lexer.script",
        "display.script",
    ];

    for file in &files {
        let path = calc_dir.join(file);
        let source =
            fs::read_to_string(&path).expect(&format!("Failed to read {}", path.display()));

        let lexer = Lexer::new(&source);
        let (tokens, lex_errors) = lexer.scan_tokens();

        assert!(
            lex_errors.is_empty(),
            "Lexer errors in {}: {:?}",
            file,
            lex_errors
        );

        let mut parser = Parser::new(tokens);
        match parser.parse() {
            Ok(_) => println!("✓ {} parsed successfully", file),
            Err(e) => panic!("Parser error in {}: {:?}", file, e),
        }
    }
}

#[test]
fn test_game_engine_example_parses() {
    let game_dir = PathBuf::from("examples/modules/game_engine");
    let files = [
        ("main.script", ""),
        ("core/engine.script", "core"),
        ("core/system.script", "core"),
        ("entities/entity.script", "entities"),
        ("entities/player.script", "entities"),
        ("entities/enemy.script", "entities"),
        ("systems/physics.script", "systems"),
        ("systems/renderer.script", "systems"),
        ("utils/vector.script", "utils"),
        ("utils/math.script", "utils"),
    ];

    for (file, subdir) in &files {
        let path = if subdir.is_empty() {
            game_dir.join(file)
        } else {
            game_dir.join(subdir).join(file)
        };

        let source =
            fs::read_to_string(&path).expect(&format!("Failed to read {}", path.display()));

        let lexer = Lexer::new(&source);
        let (tokens, lex_errors) = lexer.scan_tokens();

        assert!(
            lex_errors.is_empty(),
            "Lexer errors in {}: {:?}",
            file,
            lex_errors
        );

        let mut parser = Parser::new(tokens);
        match parser.parse() {
            Ok(_) => println!("✓ {} parsed successfully", file),
            Err(e) => panic!("Parser error in {}: {:?}", file, e),
        }
    }
}

#[test]
fn test_module_test_files_parse() {
    let test_dir = PathBuf::from("tests/modules");
    let files = [
        "math_utils.script",
        "string_utils.script",
        "geometry.script",
        "point.script",
        "circular_a.script",
        "circular_b.script",
    ];

    for file in &files {
        let path = test_dir.join(file);
        let source =
            fs::read_to_string(&path).expect(&format!("Failed to read {}", path.display()));

        let lexer = Lexer::new(&source);
        let (tokens, lex_errors) = lexer.scan_tokens();

        assert!(
            lex_errors.is_empty(),
            "Lexer errors in {}: {:?}",
            file,
            lex_errors
        );

        let mut parser = Parser::new(tokens);
        match parser.parse() {
            Ok(program) => {
                println!("✓ {} parsed successfully", file);

                // Check for import/export statements
                let mut has_imports = false;
                let mut has_exports = false;

                for stmt in &program.statements {
                    match &stmt.kind {
                        script::parser::StmtKind::Import { .. } => has_imports = true,
                        script::parser::StmtKind::Export { .. } => has_exports = true,
                        _ => {}
                    }
                }

                if file != "circular_a.script" && file != "circular_b.script" {
                    assert!(has_exports, "{} should have export statements", file);
                }

                if file == "geometry.script" || file.starts_with("circular") {
                    assert!(has_imports, "{} should have import statements", file);
                }
            }
            Err(e) => panic!("Parser error in {}: {:?}", file, e),
        }
    }
}

#[test]
fn test_modules_example_file() {
    let path = PathBuf::from("examples/modules_example.script");
    let source = fs::read_to_string(&path).expect("Failed to read modules_example.script");

    let lexer = Lexer::new(&source);
    let (tokens, lex_errors) = lexer.scan_tokens();

    assert!(lex_errors.is_empty(), "Lexer errors: {:?}", lex_errors);

    let mut parser = Parser::new(tokens);
    let program = parser
        .parse()
        .expect("Failed to parse modules_example.script");

    // Verify it has import statements
    let import_count = program
        .statements
        .iter()
        .filter(|stmt| matches!(&stmt.kind, script::parser::StmtKind::Import { .. }))
        .count();

    assert!(
        import_count >= 4,
        "modules_example.script should have at least 4 import statements, found {}",
        import_count
    );
}
