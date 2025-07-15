use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::semantic::{SemanticAnalyzer, SymbolKind};
use crate::stdlib::StdLib;
use crate::types::Type;
use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionResponse, CompletionTriggerKind, Documentation,
    MarkupContent, MarkupKind, Position,
};

/// Keywords in the Script language
const KEYWORDS: &[(&str, &str)] = &[
    ("fn", "Defines a function"),
    ("let", "Declares a variable"),
    ("if", "Conditional statement"),
    ("else", "Alternative branch of if statement"),
    ("while", "Loop that continues while condition is true"),
    ("for", "Iterator-based loop"),
    ("return", "Returns a value from a function"),
    ("true", "Boolean true literal"),
    ("false", "Boolean false literal"),
    ("print", "Built-in print statement"),
    ("match", "Pattern matching expression"),
    ("async", "Marks a function as asynchronous"),
    ("await", "Waits for an async operation"),
    ("import", "Imports symbols from another module"),
    ("export", "Exports symbols from a module"),
    ("from", "Specifies module in import statement"),
    ("as", "Renames imported symbols"),
    ("in", "Used in for loops and list comprehensions"),
];

/// Generate completion items for a document at a specific position
pub fn generate_completions(
    content: &str,
    position: Position,
    _trigger_kind: CompletionTriggerKind,
) -> CompletionResponse {
    let mut items = Vec::new();

    // Get the context at the cursor position
    let context = get_completion_context(content, position);

    // Add appropriate completions based on context
    match context {
        CompletionContext::TopLevel => {
            items.extend(get_keyword_completions());
            items.extend(get_stdlib_completions());
        }
        CompletionContext::Expression { prefix } => {
            items.extend(
                get_keyword_completions()
                    .into_iter()
                    .filter(|item| item.label.starts_with(&prefix)),
            );
            items.extend(
                get_stdlib_completions()
                    .into_iter()
                    .filter(|item| item.label.starts_with(&prefix)),
            );
            items.extend(get_variable_completions(content, position, &prefix));
        }
        CompletionContext::MemberAccess { object, prefix } => {
            items.extend(get_member_completions(&object, &prefix));
        }
        CompletionContext::Import => {
            items.extend(get_module_completions());
        }
        CompletionContext::Type => {
            items.extend(get_type_completions());
        }
    }

    CompletionResponse::Array(items)
}

/// Context for completion
#[derive(Debug)]
enum CompletionContext {
    TopLevel,
    Expression { prefix: String },
    MemberAccess { object: String, prefix: String },
    Import,
    Type,
}

/// Determine the completion context at a given position
fn get_completion_context(content: &str, position: Position) -> CompletionContext {
    let lines: Vec<&str> = content.lines().collect();
    if position.line as usize >= lines.len() {
        return CompletionContext::TopLevel;
    }

    let line = lines[position.line as usize];
    let col = position.character as usize;

    // Check if we're after a dot (member access)
    if let Some(dot_pos) = line[..col].rfind('.') {
        let object_part = &line[..dot_pos];
        let member_part = &line[dot_pos + 1..col];

        // Find the object identifier
        if let Some(object) = extract_identifier_before(object_part) {
            return CompletionContext::MemberAccess {
                object,
                prefix: member_part.to_string(),
            };
        }
    }

    // Check if we're in an import statement
    if line.trim_start().starts_with("import") || line.trim_start().starts_with("from") {
        return CompletionContext::Import;
    }

    // Check if we're in a type position (after colon)
    if line[..col].contains(':') && !line[..col].contains("::") {
        let after_colon = line[..col].rsplit(':').next().unwrap_or("");
        if after_colon
            .trim()
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_')
        {
            return CompletionContext::Type;
        }
    }

    // Otherwise, we're in an expression context
    let prefix = extract_current_word(&line[..col]);
    CompletionContext::Expression { prefix }
}

/// Extract the identifier before a position
fn extract_identifier_before(text: &str) -> Option<String> {
    let trimmed = text.trim_end();
    let chars: Vec<char> = trimmed.chars().collect();

    let mut end = chars.len();
    while end > 0 && chars[end - 1].is_whitespace() {
        end -= 1;
    }

    let mut start = end;
    while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_') {
        start -= 1;
    }

    if start < end {
        Some(chars[start..end].iter().collect())
    } else {
        None
    }
}

/// Extract the current word being typed
fn extract_current_word(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut start = chars.len();

    while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_') {
        start -= 1;
    }

    chars[start..].iter().collect()
}

/// Get keyword completions
fn get_keyword_completions() -> Vec<CompletionItem> {
    KEYWORDS
        .iter()
        .map(|(keyword, description)| CompletionItem {
            label: keyword.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("keyword".to_string()),
            documentation: Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: description.to_string(),
            })),
            ..Default::default()
        })
        .collect()
}

/// Get standard library function completions
fn get_stdlib_completions() -> Vec<CompletionItem> {
    let stdlib = StdLib::new();
    let mut items = Vec::new();

    for func_name in stdlib.function_names() {
        if let Some(func) = stdlib.get_function(func_name) {
            let detail = format_function_signature(func_name, &func.signature);

            items.push(CompletionItem {
                label: func_name.to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some(detail),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: format!("Built-in function `{}`", func_name),
                })),
                insert_text: Some(format!("{}(", func_name)),
                ..Default::default()
            });
        }
    }

    items
}

/// Get variable completions from the current scope
fn get_variable_completions(
    content: &str,
    _position: Position,
    prefix: &str,
) -> Vec<CompletionItem> {
    let mut items = Vec::new();

    // Parse the content
    let lexer = match Lexer::new(content) {
        Ok(lexer) => lexer,
        Err(_) => return items, // Return empty items if lexer initialization fails
    };
    let (tokens, errors) = lexer.scan_tokens();
    if !errors.is_empty() {
        return items; // Return empty items if there are lexer errors
    }

    let mut parser = Parser::new(tokens);
    let program = match parser.parse() {
        Ok(program) => program,
        Err(_) => return items,
    };

    // Run semantic analysis to get symbol information
    let mut analyzer = SemanticAnalyzer::new();
    let _ = analyzer.analyze_program(&program); // Ignore errors for completion

    // Get all symbols from the symbol table and filter based on position
    // For now, we'll get all symbols in scope (this is a simplified approach)
    let symbol_table = analyzer.symbol_table();

    // Get current scope symbols
    let symbols = symbol_table.get_current_scope_symbols();

    for symbol in symbols {
        if symbol.name.starts_with(prefix) {
            let kind = match &symbol.kind {
                SymbolKind::Variable { .. } => CompletionItemKind::VARIABLE,
                SymbolKind::Function(_) => CompletionItemKind::FUNCTION,
                SymbolKind::Parameter => CompletionItemKind::VARIABLE,
                _ => CompletionItemKind::VARIABLE,
            };

            let detail = format_type(&symbol.ty);

            items.push(CompletionItem {
                label: symbol.name.clone(),
                kind: Some(kind),
                detail: Some(detail),
                documentation: None,
                ..Default::default()
            });
        }
    }

    // Also add global symbols by looking them up
    for (keyword, _) in KEYWORDS {
        if keyword.starts_with(&prefix) {
            // Skip keywords that are already in the list
            if !items.iter().any(|item| item.label == *keyword) {
                // Try to look up as a potential variable name
                if let Some(symbol) = symbol_table.lookup(keyword) {
                    let kind = match &symbol.kind {
                        SymbolKind::Variable { .. } => CompletionItemKind::VARIABLE,
                        SymbolKind::Function(_) => CompletionItemKind::FUNCTION,
                        SymbolKind::Parameter => CompletionItemKind::VARIABLE,
                        _ => CompletionItemKind::VARIABLE,
                    };

                    let detail = format_type(&symbol.ty);

                    items.push(CompletionItem {
                        label: symbol.name.clone(),
                        kind: Some(kind),
                        detail: Some(detail),
                        documentation: None,
                        ..Default::default()
                    });
                }
            }
        }
    }

    items
}

/// Get member completions for an object
fn get_member_completions(object: &str, prefix: &str) -> Vec<CompletionItem> {
    let mut items = Vec::new();

    // String methods
    if object == "string" || object.ends_with("\"") || object.ends_with("'") {
        let string_methods = vec![
            ("len", "() -> i32", "Returns the length of the string"),
            (
                "to_uppercase",
                "() -> string",
                "Converts the string to uppercase",
            ),
            (
                "to_lowercase",
                "() -> string",
                "Converts the string to lowercase",
            ),
            (
                "trim",
                "() -> string",
                "Removes leading and trailing whitespace",
            ),
            (
                "split",
                "(separator: string) -> Array<string>",
                "Splits the string by separator",
            ),
            (
                "contains",
                "(substring: string) -> bool",
                "Checks if string contains substring",
            ),
            (
                "replace",
                "(from: string, to: string) -> string",
                "Replaces all occurrences",
            ),
        ];

        for (method, signature, doc) in string_methods {
            if method.starts_with(prefix) {
                items.push(CompletionItem {
                    label: method.to_string(),
                    kind: Some(CompletionItemKind::METHOD),
                    detail: Some(signature.to_string()),
                    documentation: Some(Documentation::MarkupContent(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: doc.to_string(),
                    })),
                    insert_text: Some(format!("{}(", method)),
                    ..Default::default()
                });
            }
        }
    }

    // Array methods
    if object.contains('[') || object == "Vec" || object == "Array" {
        let array_methods = vec![
            ("len", "() -> i32", "Returns the number of elements"),
            ("push", "(item: T) -> unit", "Adds an element to the end"),
            (
                "pop",
                "() -> Option<T>",
                "Removes and returns the last element",
            ),
            ("get", "(index: i32) -> Option<T>", "Gets element at index"),
        ];

        for (method, signature, doc) in array_methods {
            if method.starts_with(prefix) {
                items.push(CompletionItem {
                    label: method.to_string(),
                    kind: Some(CompletionItemKind::METHOD),
                    detail: Some(signature.to_string()),
                    documentation: Some(Documentation::MarkupContent(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: doc.to_string(),
                    })),
                    insert_text: Some(format!("{}(", method)),
                    ..Default::default()
                });
            }
        }
    }

    items
}

/// Get module name completions
fn get_module_completions() -> Vec<CompletionItem> {
    // In a real implementation, this would scan available modules
    vec![
        CompletionItem {
            label: "std".to_string(),
            kind: Some(CompletionItemKind::MODULE),
            detail: Some("Standard library".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "math".to_string(),
            kind: Some(CompletionItemKind::MODULE),
            detail: Some("Mathematical functions".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "io".to_string(),
            kind: Some(CompletionItemKind::MODULE),
            detail: Some("Input/output operations".to_string()),
            ..Default::default()
        },
    ]
}

/// Get type completions
fn get_type_completions() -> Vec<CompletionItem> {
    let types = vec![
        ("i32", "32-bit signed integer"),
        ("f32", "32-bit floating point"),
        ("bool", "Boolean value"),
        ("string", "UTF-8 string"),
        ("unit", "Unit type (void)"),
        ("Array", "Dynamic array"),
        ("HashMap", "Hash map"),
        ("Option", "Optional value"),
        ("Result", "Result type for error handling"),
    ];

    types
        .iter()
        .map(|(ty, description)| CompletionItem {
            label: ty.to_string(),
            kind: Some(CompletionItemKind::CLASS),
            detail: Some("type".to_string()),
            documentation: Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: description.to_string(),
            })),
            ..Default::default()
        })
        .collect()
}

/// Format a function signature for display
fn format_function_signature(name: &str, ty: &Type) -> String {
    match ty {
        Type::Function { params, ret } => {
            let param_str = params
                .iter()
                .map(|p| format_type(p))
                .collect::<Vec<_>>()
                .join(", ");
            format!("fn {}({}) -> {}", name, param_str, format_type(ret))
        }
        _ => format!("fn {name}"),
    }
}

/// Format a type for display
fn format_type(ty: &Type) -> String {
    match ty {
        Type::I32 => "i32".to_string(),
        Type::F32 => "f32".to_string(),
        Type::Bool => "bool".to_string(),
        Type::String => "string".to_string(),
        Type::Array(inner) => format!("Array<{}>", format_type(inner)),
        Type::Option(inner) => format!("Option<{}>", format_type(inner)),
        Type::Result { ok, err } => format!("Result<{}, {}>", format_type(ok), format_type(err)),
        Type::Function { params, ret } => {
            let param_str = params
                .iter()
                .map(|p| format_type(p))
                .collect::<Vec<_>>()
                .join(", ");
            format!("({}) -> {}", param_str, format_type(ret))
        }
        Type::Named(name) => name.clone(),
        Type::Unknown => "?".to_string(),
        Type::Never => "never".to_string(),
        Type::Future(inner) => format!("Future<{}>", format_type(inner)),
        Type::TypeVar(id) => format!("T{id}"),
        Type::Generic { name, args } => {
            if args.is_empty() {
                name.clone()
            } else {
                let arg_str = args
                    .iter()
                    .map(|arg| format_type(arg))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}<{}>", name, arg_str)
            }
        }
        Type::TypeParam(name) => name.clone(),
        Type::Tuple(types) => {
            let type_str = types
                .iter()
                .map(|t| format_type(t))
                .collect::<Vec<_>>()
                .join(", ");
            format!("({})", type_str)
        }
        Type::Reference { mutable, inner } => {
            if *mutable {
                format!("&mut {}", format_type(inner))
            } else {
                format!("&{}", format_type(inner))
            }
        }
        Type::Struct { name, .. } => name.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_current_word() {
        assert_eq!(extract_current_word("let x"), "x");
        assert_eq!(extract_current_word("let my_var"), "my_var");
        assert_eq!(extract_current_word("print"), "print");
        assert_eq!(extract_current_word(""), "");
        assert_eq!(extract_current_word("  "), "");
    }

    #[test]
    fn test_extract_identifier_before() {
        assert_eq!(extract_identifier_before("obj."), Some("obj".to_string()));
        assert_eq!(
            extract_identifier_before("my_object ."),
            Some("my_object".to_string())
        );
        assert_eq!(extract_identifier_before(" "), None);
    }

    #[test]
    fn test_completion_context() {
        let pos = Position {
            line: 0,
            character: 4,
        };
        match get_completion_context("obj.", pos) {
            CompletionContext::MemberAccess { object, prefix } => {
                assert_eq!(object, "obj");
                assert_eq!(prefix, "");
            }
            _ => panic!("Expected member access context"),
        }

        let pos = Position {
            line: 0,
            character: 7,
        };
        match get_completion_context("let x: ", pos) {
            CompletionContext::Type => {}
            _ => panic!("Expected type context"),
        }
    }

    #[test]
    fn test_keyword_completions() {
        let completions = get_keyword_completions();
        assert!(completions.iter().any(|c| c.label == "fn"));
        assert!(completions.iter().any(|c| c.label == "let"));
        assert!(completions
            .iter()
            .all(|c| c.kind == Some(CompletionItemKind::KEYWORD)));
    }

    #[test]
    fn test_stdlib_completions() {
        let completions = get_stdlib_completions();

        // Check for some common stdlib functions
        assert!(completions.iter().any(|c| c.label == "print"));
        assert!(completions.iter().any(|c| c.label == "println"));
        assert!(completions.iter().any(|c| c.label == "abs"));
        assert!(completions.iter().any(|c| c.label == "sqrt"));

        // Check that they're marked as functions
        assert!(completions
            .iter()
            .all(|c| c.kind == Some(CompletionItemKind::FUNCTION)));

        // Check that they have insert text with parentheses
        let print_completion = completions.iter().find(|c| c.label == "print").unwrap();
        assert_eq!(print_completion.insert_text, Some("print(".to_string()));
    }

    #[test]
    fn test_type_completions() {
        let completions = get_type_completions();

        // Check for basic types
        assert!(completions.iter().any(|c| c.label == "i32"));
        assert!(completions.iter().any(|c| c.label == "f32"));
        assert!(completions.iter().any(|c| c.label == "bool"));
        assert!(completions.iter().any(|c| c.label == "string"));

        // Check that they're marked as classes (types)
        assert!(completions
            .iter()
            .all(|c| c.kind == Some(CompletionItemKind::CLASS)));
    }

    #[test]
    fn test_generate_completions_keywords() {
        let content = "f";
        let pos = Position {
            line: 0,
            character: 1,
        };

        match generate_completions(content, pos, CompletionTriggerKind::INVOKED) {
            CompletionResponse::Array(items) => {
                // Should include keywords starting with 'f'
                assert!(items.iter().any(|c| c.label == "fn"));
                assert!(items.iter().any(|c| c.label == "for"));
                assert!(items.iter().any(|c| c.label == "false"));
                assert!(items.iter().any(|c| c.label == "from"));

                // Should not include keywords not starting with 'f'
                assert!(!items.iter().any(|c| c.label == "let"));
                assert!(!items.iter().any(|c| c.label == "while"));
            }
            _ => panic!("Expected array response"),
        }
    }

    #[test]
    fn test_generate_completions_member_access() {
        let content = "str.";
        let pos = Position {
            line: 0,
            character: 4,
        };

        match generate_completions(content, pos, CompletionTriggerKind::TRIGGER_CHARACTER) {
            CompletionResponse::Array(items) => {
                // Should include string methods
                assert!(items.iter().any(|c| c.label == "len"));
                assert!(items.iter().any(|c| c.label == "to_uppercase"));
                assert!(items.iter().any(|c| c.label == "trim"));

                // All should be methods
                assert!(items
                    .iter()
                    .all(|c| c.kind == Some(CompletionItemKind::METHOD)));
            }
            _ => panic!("Expected array response"),
        }
    }

    #[test]
    fn test_format_type() {
        assert_eq!(format_type(&Type::I32), "i32");
        assert_eq!(format_type(&Type::String), "string");
        assert_eq!(format_type(&Type::Array(Box::new(Type::I32))), "Array<i32>");
        assert_eq!(
            format_type(&Type::Option(Box::new(Type::String))),
            "Option<string>"
        );
        assert_eq!(
            format_type(&Type::Function {
                params: vec![Type::I32, Type::String],
                ret: Box::new(Type::Bool)
            }),
            "(i32, string) -> bool"
        );
    }
}
