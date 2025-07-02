use script::lsp::definition::goto_definition;
use tower_lsp::lsp_types::{Position, Url};

fn main() {
    // Test 1: Variable definition
    let content = r#"let x = 42;
let y = x + 1;"#;

    let position = Position {
        line: 1,
        character: 8, // Position on 'x' in 'x + 1'
    };

    let uri = Url::parse("file:///test.script").unwrap();
    match goto_definition(content, position, &uri) {
        Some(location) => {
            println!(
                "✓ Test 1 passed: Found definition at line {}",
                location.range.start.line
            );
        }
        None => {
            println!("✗ Test 1 failed: Definition not found");
        }
    }

    // Test 2: Function definition
    let content = r#"fn add(x: i32, y: i32) -> i32 {
    x + y
}

let result = add(1, 2);"#;

    let position = Position {
        line: 4,
        character: 13, // Position on 'add' in function call
    };

    match goto_definition(content, position, &uri) {
        Some(location) => {
            println!(
                "✓ Test 2 passed: Found definition at line {}",
                location.range.start.line
            );
        }
        None => {
            println!("✗ Test 2 failed: Definition not found");
        }
    }

    // Test 3: Not found (position on literal)
    let content = r#"let x = 42;"#;
    let position = Position {
        line: 0,
        character: 9, // Position on '42'
    };

    match goto_definition(content, position, &uri) {
        Some(_) => {
            println!("✗ Test 3 failed: Should not find definition for literal");
        }
        None => {
            println!("✓ Test 3 passed: Correctly returned None for literal");
        }
    }
}
