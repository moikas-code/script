//! Pattern exhaustiveness checking for match expressions
//!
//! This module implements an algorithm to verify that match expressions
//! cover all possible cases, ensuring no runtime panics from missing patterns.

use crate::parser::{Literal, MatchArm, Pattern, PatternKind};
use crate::semantic::symbol::EnumInfo;
use crate::semantic::{SymbolKind, SymbolTable};
use crate::source::Span;
use crate::types::Type;
use std::collections::HashSet;

/// Result of exhaustiveness checking
#[derive(Debug)]
pub struct ExhaustivenessResult {
    /// Whether the patterns are exhaustive
    pub is_exhaustive: bool,
    /// Missing patterns (if any)
    pub missing_patterns: Vec<String>,
    /// Redundant patterns (if any)
    pub redundant_patterns: Vec<(usize, Span)>,
    /// Whether any patterns have guards (affects exhaustiveness checking)
    pub has_guards: bool,
}

/// Pattern matrix for exhaustiveness checking
#[derive(Debug)]
struct PatternMatrix {
    /// Each row is a pattern arm
    rows: Vec<PatternRow>,
}

#[derive(Debug)]
struct PatternRow {
    /// The patterns in this row
    patterns: Vec<Pattern>,
    /// The original arm index (for error reporting)
    arm_index: usize,
    /// The span of the original pattern
    span: Span,
}

/// Check if a set of match arms is exhaustive for a given type
pub fn check_exhaustiveness(
    arms: &[MatchArm],
    scrutinee_type: &Type,
    _scrutinee_span: Span,
    symbol_table: &SymbolTable,
) -> ExhaustivenessResult {
    // Build the pattern matrix
    let mut matrix = PatternMatrix::new();

    for (index, arm) in arms.iter().enumerate() {
        matrix.add_row(vec![arm.pattern.clone()], index, arm.pattern.span);
    }

    // Check for wildcard or catch-all pattern WITHOUT guards
    // A pattern with a guard doesn't guarantee exhaustiveness
    let has_wildcard = arms.iter().any(|arm| {
        arm.guard.is_none()
            && (matches!(arm.pattern.kind, PatternKind::Wildcard)
                || matches!(arm.pattern.kind, PatternKind::Identifier(_)))
    });

    // Simple exhaustiveness check for now
    // TODO: Implement full pattern exhaustiveness algorithm
    let is_exhaustive =
        has_wildcard || check_simple_exhaustiveness(arms, scrutinee_type, symbol_table);

    // Find redundant patterns
    let redundant_patterns = find_redundant_patterns(arms);

    // Generate missing patterns
    let missing_patterns = if !is_exhaustive {
        generate_missing_patterns(arms, scrutinee_type, symbol_table)
    } else {
        vec![]
    };

    // Check if any patterns have guards
    let has_guards = arms.iter().any(|arm| arm.guard.is_some());

    ExhaustivenessResult {
        is_exhaustive,
        missing_patterns,
        redundant_patterns,
        has_guards,
    }
}

/// Simple exhaustiveness check for basic types
fn check_simple_exhaustiveness(
    arms: &[MatchArm],
    scrutinee_type: &Type,
    symbol_table: &SymbolTable,
) -> bool {
    match scrutinee_type {
        Type::Bool => {
            // Check if both true and false are covered
            let mut has_true = false;
            let mut has_false = false;

            for arm in arms {
                // Only count patterns without guards for exhaustiveness
                if arm.guard.is_none() {
                    if let PatternKind::Literal(Literal::Boolean(b)) = &arm.pattern.kind {
                        if *b {
                            has_true = true;
                        } else {
                            has_false = true;
                        }
                    }
                }
            }

            has_true && has_false
        }
        Type::I32 | Type::F32 | Type::String => {
            // For numeric and string types, we can't enumerate all values
            // So we require a wildcard or identifier pattern
            false
        }
        Type::Array(_) => {
            // Arrays require more complex pattern matching
            // For now, require a wildcard
            false
        }
        Type::Result { .. } => {
            // For Result<T, E>, we need to check Ok and Err variants
            check_result_exhaustiveness(arms)
        }
        Type::Option(_) => {
            // For Option<T>, we need to check Some and None variants
            check_option_exhaustiveness(arms)
        }
        Type::Named(name) => {
            // Check if this is an enum type
            if let Some(symbol) = symbol_table.lookup(name) {
                if let SymbolKind::Enum(enum_info) = &symbol.kind {
                    return check_enum_exhaustiveness(arms, name, enum_info);
                }
            }
            // For non-enum named types, require a wildcard
            false
        }
        _ => false,
    }
}

/// Check if enum patterns are exhaustive
fn check_enum_exhaustiveness(arms: &[MatchArm], _enum_name: &str, enum_info: &EnumInfo) -> bool {
    use std::collections::HashSet;

    // Collect all variant names
    let all_variants: HashSet<&str> = enum_info.variants.iter().map(|v| v.name.as_str()).collect();

    // Track which variants are covered
    let mut covered_variants = HashSet::new();

    for arm in arms {
        // Skip patterns with guards as they don't guarantee coverage
        if arm.guard.is_some() {
            continue;
        }

        match &arm.pattern.kind {
            PatternKind::EnumConstructor { variant, .. } => {
                covered_variants.insert(variant.as_str());
            }
            PatternKind::Wildcard | PatternKind::Identifier(_) => {
                // Wildcard or identifier patterns cover all variants
                return true;
            }
            PatternKind::Or(patterns) => {
                // Check each alternative in the or-pattern
                for pattern in patterns {
                    if let PatternKind::EnumConstructor { variant, .. } = &pattern.kind {
                        covered_variants.insert(variant.as_str());
                    }
                }
            }
            _ => {
                // Other pattern kinds don't match enum constructors
            }
        }
    }

    // Check if all variants are covered
    all_variants == covered_variants
}

/// Check if Result<T, E> patterns are exhaustive
fn check_result_exhaustiveness(arms: &[MatchArm]) -> bool {
    let mut has_ok = false;
    let mut has_err = false;

    for arm in arms {
        // Skip patterns with guards as they don't guarantee coverage
        if arm.guard.is_some() {
            continue;
        }

        match &arm.pattern.kind {
            PatternKind::EnumConstructor {
                enum_name, variant, ..
            } => {
                // Check if this is a Result variant
                if enum_name.as_deref() == Some("Result") || variant == "Ok" || variant == "Err" {
                    match variant.as_str() {
                        "Ok" => has_ok = true,
                        "Err" => has_err = true,
                        _ => {}
                    }
                }
            }
            PatternKind::Wildcard | PatternKind::Identifier(_) => {
                // Wildcard or identifier patterns cover all variants
                return true;
            }
            PatternKind::Or(patterns) => {
                // Check each alternative in the or-pattern
                for pattern in patterns {
                    if let PatternKind::EnumConstructor { variant, .. } = &pattern.kind {
                        match variant.as_str() {
                            "Ok" => has_ok = true,
                            "Err" => has_err = true,
                            _ => {}
                        }
                    }
                }
            }
            _ => {
                // Other pattern kinds don't match Result
            }
        }
    }

    has_ok && has_err
}

/// Check if Option<T> patterns are exhaustive
fn check_option_exhaustiveness(arms: &[MatchArm]) -> bool {
    let mut has_some = false;
    let mut has_none = false;

    for arm in arms {
        // Skip patterns with guards as they don't guarantee coverage
        if arm.guard.is_some() {
            continue;
        }

        match &arm.pattern.kind {
            PatternKind::EnumConstructor {
                enum_name, variant, ..
            } => {
                // Check if this is an Option variant
                if enum_name.as_deref() == Some("Option") || variant == "Some" || variant == "None"
                {
                    match variant.as_str() {
                        "Some" => has_some = true,
                        "None" => has_none = true,
                        _ => {}
                    }
                }
            }
            PatternKind::Wildcard | PatternKind::Identifier(_) => {
                // Wildcard or identifier patterns cover all variants
                return true;
            }
            PatternKind::Or(patterns) => {
                // Check each alternative in the or-pattern
                for pattern in patterns {
                    if let PatternKind::EnumConstructor { variant, .. } = &pattern.kind {
                        match variant.as_str() {
                            "Some" => has_some = true,
                            "None" => has_none = true,
                            _ => {}
                        }
                    }
                }
            }
            _ => {
                // Other pattern kinds don't match Option
            }
        }
    }

    has_some && has_none
}

/// Find patterns that are redundant (unreachable)
fn find_redundant_patterns(arms: &[MatchArm]) -> Vec<(usize, Span)> {
    let mut redundant = Vec::new();
    let mut covered_patterns = Vec::new();

    for (index, arm) in arms.iter().enumerate() {
        // Check if this pattern is subsumed by previous patterns
        if is_pattern_redundant(&arm.pattern, &covered_patterns, arm.guard.is_some()) {
            redundant.push((index, arm.pattern.span));
        }

        // Only add to covered patterns if there's no guard
        // Patterns with guards don't fully cover their pattern space
        if arm.guard.is_none() {
            covered_patterns.push(&arm.pattern);
        }
    }

    redundant
}

/// Check if a pattern is redundant given previously seen patterns
fn is_pattern_redundant(pattern: &Pattern, previous: &[&Pattern], has_guard: bool) -> bool {
    // A pattern with a guard is never redundant because the guard might fail
    if has_guard {
        return false;
    }

    // A pattern is redundant if it's subsumed by any previous pattern
    for prev in previous {
        if pattern_subsumes(prev, pattern) {
            return true;
        }
    }

    false
}

/// Check if pattern1 subsumes pattern2 (i.e., pattern1 matches everything pattern2 does)
fn pattern_subsumes(pattern1: &Pattern, pattern2: &Pattern) -> bool {
    match (&pattern1.kind, &pattern2.kind) {
        // Wildcard and identifier patterns subsume everything
        (PatternKind::Wildcard, _) | (PatternKind::Identifier(_), _) => true,

        // Same literals
        (PatternKind::Literal(lit1), PatternKind::Literal(lit2)) => lit1 == lit2,

        // Array patterns
        (PatternKind::Array(pats1), PatternKind::Array(pats2)) => {
            pats1.len() == pats2.len()
                && pats1
                    .iter()
                    .zip(pats2.iter())
                    .all(|(p1, p2)| pattern_subsumes(p1, p2))
        }

        // Enum constructor patterns
        (
            PatternKind::EnumConstructor {
                enum_name: en1,
                variant: v1,
                args: args1,
            },
            PatternKind::EnumConstructor {
                enum_name: en2,
                variant: v2,
                args: args2,
            },
        ) => {
            // Must be the same variant
            v1 == v2 && en1 == en2 &&
            // And arguments must match
            match (args1, args2) {
                (None, None) => true,
                (Some(a1), Some(a2)) => {
                    a1.len() == a2.len() &&
                    a1.iter().zip(a2.iter()).all(|(p1, p2)| pattern_subsumes(p1, p2))
                }
                _ => false,
            }
        }

        // Or patterns
        (PatternKind::Or(pats), _) => {
            // An or-pattern subsumes if any of its alternatives subsume
            pats.iter().any(|p| pattern_subsumes(p, pattern2))
        }
        (_, PatternKind::Or(pats)) => {
            // pattern1 subsumes an or-pattern if it subsumes all alternatives
            pats.iter().all(|p| pattern_subsumes(pattern1, p))
        }

        _ => false,
    }
}

/// Generate descriptions of missing patterns
fn generate_missing_patterns(
    arms: &[MatchArm],
    scrutinee_type: &Type,
    symbol_table: &SymbolTable,
) -> Vec<String> {
    let mut missing = Vec::new();

    match scrutinee_type {
        Type::Bool => {
            let mut has_true = false;
            let mut has_false = false;

            for arm in arms {
                if let PatternKind::Literal(Literal::Boolean(b)) = &arm.pattern.kind {
                    if *b {
                        has_true = true;
                    } else {
                        has_false = true;
                    }
                }
            }

            if !has_true {
                missing.push("true".to_string());
            }
            if !has_false {
                missing.push("false".to_string());
            }
        }
        Type::I32 => {
            missing.push("_ (or any integer pattern)".to_string());
        }
        Type::F32 => {
            missing.push("_ (or any float pattern)".to_string());
        }
        Type::String => {
            missing.push("_ (or any string pattern)".to_string());
        }
        Type::Array(_) => {
            missing.push("_ (or any array pattern)".to_string());
        }
        Type::Result { .. } => {
            let mut has_ok = false;
            let mut has_err = false;

            for arm in arms {
                if arm.guard.is_none() {
                    match &arm.pattern.kind {
                        PatternKind::EnumConstructor { variant, .. } => match variant.as_str() {
                            "Ok" => has_ok = true,
                            "Err" => has_err = true,
                            _ => {}
                        },
                        PatternKind::Or(patterns) => {
                            for pattern in patterns {
                                if let PatternKind::EnumConstructor { variant, .. } = &pattern.kind
                                {
                                    match variant.as_str() {
                                        "Ok" => has_ok = true,
                                        "Err" => has_err = true,
                                        _ => {}
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }

            if !has_ok {
                missing.push("Ok(_)".to_string());
            }
            if !has_err {
                missing.push("Err(_)".to_string());
            }
        }
        Type::Option(_) => {
            let mut has_some = false;
            let mut has_none = false;

            for arm in arms {
                if arm.guard.is_none() {
                    match &arm.pattern.kind {
                        PatternKind::EnumConstructor { variant, .. } => match variant.as_str() {
                            "Some" => has_some = true,
                            "None" => has_none = true,
                            _ => {}
                        },
                        PatternKind::Or(patterns) => {
                            for pattern in patterns {
                                if let PatternKind::EnumConstructor { variant, .. } = &pattern.kind
                                {
                                    match variant.as_str() {
                                        "Some" => has_some = true,
                                        "None" => has_none = true,
                                        _ => {}
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }

            if !has_some {
                missing.push("Some(_)".to_string());
            }
            if !has_none {
                missing.push("None".to_string());
            }
        }
        Type::Named(name) => {
            // Check if this is an enum type
            if let Some(symbol) = symbol_table.lookup(name) {
                if let SymbolKind::Enum(enum_info) = &symbol.kind {
                    // Find missing enum variants
                    let mut covered_variants = HashSet::new();

                    for arm in arms {
                        if arm.guard.is_none() {
                            match &arm.pattern.kind {
                                PatternKind::EnumConstructor { variant, .. } => {
                                    covered_variants.insert(variant.as_str());
                                }
                                PatternKind::Or(patterns) => {
                                    for pattern in patterns {
                                        if let PatternKind::EnumConstructor { variant, .. } =
                                            &pattern.kind
                                        {
                                            covered_variants.insert(variant.as_str());
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }

                    // Generate missing patterns for uncovered variants
                    for variant_info in &enum_info.variants {
                        if !covered_variants.contains(variant_info.name.as_str()) {
                            use crate::semantic::symbol::EnumVariantType;
                            let pattern = match &variant_info.variant_type {
                                EnumVariantType::Unit => variant_info.name.clone(),
                                EnumVariantType::Tuple(types) => {
                                    let wildcards = vec!["_"; types.len()].join(", ");
                                    format!("{}({})", variant_info.name, wildcards)
                                }
                                EnumVariantType::Struct(_fields) => {
                                    format!("{} {{ .. }}", variant_info.name)
                                }
                            };
                            missing.push(pattern);
                        }
                    }

                    // If no specific variants are missing, suggest wildcard
                    if missing.is_empty() {
                        missing.push("_ (wildcard pattern)".to_string());
                    }
                } else {
                    missing.push("_ (wildcard pattern)".to_string());
                }
            } else {
                missing.push("_ (wildcard pattern)".to_string());
            }
        }
        _ => {
            missing.push("_ (wildcard pattern)".to_string());
        }
    }

    missing
}

impl PatternMatrix {
    fn new() -> Self {
        PatternMatrix { rows: Vec::new() }
    }

    fn add_row(&mut self, patterns: Vec<Pattern>, arm_index: usize, span: Span) {
        self.rows.push(PatternRow {
            patterns,
            arm_index,
            span,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{Expr, ExprKind, MatchArm, Pattern, PatternKind};
    use crate::source::Span;

    fn dummy_span() -> Span {
        Span::dummy()
    }

    fn dummy_expr() -> Expr {
        Expr {
            kind: ExprKind::Literal(Literal::Number(0.0)),
            span: dummy_span(),
            id: 0,
        }
    }

    #[test]
    fn test_bool_exhaustiveness() {
        // Exhaustive: true and false
        let arms = vec![
            MatchArm {
                pattern: Pattern {
                    kind: PatternKind::Literal(Literal::Boolean(true)),
                    span: dummy_span(),
                },
                guard: None,
                body: dummy_expr(),
            },
            MatchArm {
                pattern: Pattern {
                    kind: PatternKind::Literal(Literal::Boolean(false)),
                    span: dummy_span(),
                },
                guard: None,
                body: dummy_expr(),
            },
        ];

        let symbol_table = SymbolTable::new();
        let result = check_exhaustiveness(&arms, &Type::Bool, dummy_span(), &symbol_table);
        assert!(result.is_exhaustive);
        assert!(result.missing_patterns.is_empty());
        assert!(!result.has_guards);
    }

    #[test]
    fn test_bool_non_exhaustive() {
        // Non-exhaustive: only true
        let arms = vec![MatchArm {
            pattern: Pattern {
                kind: PatternKind::Literal(Literal::Boolean(true)),
                span: dummy_span(),
            },
            guard: None,
            body: dummy_expr(),
        }];

        let symbol_table = SymbolTable::new();
        let result = check_exhaustiveness(&arms, &Type::Bool, dummy_span(), &symbol_table);
        assert!(!result.is_exhaustive);
        assert_eq!(result.missing_patterns, vec!["false"]);
    }

    #[test]
    fn test_wildcard_exhaustive() {
        // Exhaustive: wildcard pattern
        let arms = vec![
            MatchArm {
                pattern: Pattern {
                    kind: PatternKind::Literal(Literal::Number(1.0)),
                    span: dummy_span(),
                },
                guard: None,
                body: dummy_expr(),
            },
            MatchArm {
                pattern: Pattern {
                    kind: PatternKind::Wildcard,
                    span: dummy_span(),
                },
                guard: None,
                body: dummy_expr(),
            },
        ];

        let symbol_table = SymbolTable::new();
        let result = check_exhaustiveness(&arms, &Type::I32, dummy_span(), &symbol_table);
        assert!(result.is_exhaustive);
    }

    #[test]
    fn test_redundant_pattern() {
        // Redundant: wildcard after wildcard
        let arms = vec![
            MatchArm {
                pattern: Pattern {
                    kind: PatternKind::Wildcard,
                    span: dummy_span(),
                },
                guard: None,
                body: dummy_expr(),
            },
            MatchArm {
                pattern: Pattern {
                    kind: PatternKind::Literal(Literal::Number(1.0)),
                    span: dummy_span(),
                },
                guard: None,
                body: dummy_expr(),
            },
        ];

        let symbol_table = SymbolTable::new();
        let result = check_exhaustiveness(&arms, &Type::I32, dummy_span(), &symbol_table);
        assert!(result.is_exhaustive);
        assert_eq!(result.redundant_patterns.len(), 1);
        assert_eq!(result.redundant_patterns[0].0, 1); // Second pattern is redundant
    }

    #[test]
    fn test_enum_exhaustiveness() {
        use crate::semantic::symbol::{EnumVariantInfo, EnumVariantType, SymbolId};

        // Create a simple Option enum
        let mut symbol_table = SymbolTable::new();
        let enum_info = EnumInfo {
            generic_params: None,
            variants: vec![
                EnumVariantInfo {
                    name: "Some".to_string(),
                    variant_type: EnumVariantType::Tuple(vec![Type::I32]),
                },
                EnumVariantInfo {
                    name: "None".to_string(),
                    variant_type: EnumVariantType::Unit,
                },
            ],
            where_clause: None,
        };

        // Register the enum
        symbol_table
            .define_enum("Option".to_string(), enum_info, dummy_span())
            .unwrap();

        // Test exhaustive patterns
        let arms = vec![
            MatchArm {
                pattern: Pattern {
                    kind: PatternKind::EnumConstructor {
                        enum_name: None,
                        variant: "Some".to_string(),
                        args: Some(vec![Pattern {
                            kind: PatternKind::Wildcard,
                            span: dummy_span(),
                        }]),
                    },
                    span: dummy_span(),
                },
                guard: None,
                body: dummy_expr(),
            },
            MatchArm {
                pattern: Pattern {
                    kind: PatternKind::EnumConstructor {
                        enum_name: None,
                        variant: "None".to_string(),
                        args: None,
                    },
                    span: dummy_span(),
                },
                guard: None,
                body: dummy_expr(),
            },
        ];

        let result = check_exhaustiveness(
            &arms,
            &Type::Named("Option".to_string()),
            dummy_span(),
            &symbol_table,
        );
        assert!(result.is_exhaustive);
        assert!(result.missing_patterns.is_empty());
    }

    #[test]
    fn test_enum_non_exhaustive() {
        use crate::semantic::symbol::{EnumVariantInfo, EnumVariantType, SymbolId};

        // Create a Result enum
        let mut symbol_table = SymbolTable::new();
        let enum_info = EnumInfo {
            generic_params: None,
            variants: vec![
                EnumVariantInfo {
                    name: "Ok".to_string(),
                    variant_type: EnumVariantType::Tuple(vec![Type::String]),
                },
                EnumVariantInfo {
                    name: "Err".to_string(),
                    variant_type: EnumVariantType::Tuple(vec![Type::String]),
                },
            ],
            where_clause: None,
        };

        symbol_table
            .define_enum("Result".to_string(), enum_info, dummy_span())
            .unwrap();

        // Test non-exhaustive patterns (only Ok case)
        let arms = vec![MatchArm {
            pattern: Pattern {
                kind: PatternKind::EnumConstructor {
                    enum_name: None,
                    variant: "Ok".to_string(),
                    args: Some(vec![Pattern {
                        kind: PatternKind::Identifier("value".to_string()),
                        span: dummy_span(),
                    }]),
                },
                span: dummy_span(),
            },
            guard: None,
            body: dummy_expr(),
        }];

        let result = check_exhaustiveness(
            &arms,
            &Type::Named("Result".to_string()),
            dummy_span(),
            &symbol_table,
        );
        assert!(!result.is_exhaustive);
        assert_eq!(result.missing_patterns.len(), 1);
        assert_eq!(result.missing_patterns[0], "Err(_)");
    }

    #[test]
    fn test_enum_with_or_patterns() {
        use crate::semantic::symbol::{EnumVariantInfo, EnumVariantType, SymbolId};

        // Create an enum with multiple variants
        let mut symbol_table = SymbolTable::new();
        let enum_info = EnumInfo {
            generic_params: None,
            variants: vec![
                EnumVariantInfo {
                    name: "A".to_string(),
                    variant_type: EnumVariantType::Unit,
                },
                EnumVariantInfo {
                    name: "B".to_string(),
                    variant_type: EnumVariantType::Unit,
                },
                EnumVariantInfo {
                    name: "C".to_string(),
                    variant_type: EnumVariantType::Unit,
                },
            ],
            where_clause: None,
        };

        symbol_table
            .define_enum("ABC".to_string(), enum_info, dummy_span())
            .unwrap();

        // Test with or-pattern covering multiple variants
        let arms = vec![
            MatchArm {
                pattern: Pattern {
                    kind: PatternKind::Or(vec![
                        Pattern {
                            kind: PatternKind::EnumConstructor {
                                enum_name: None,
                                variant: "A".to_string(),
                                args: None,
                            },
                            span: dummy_span(),
                        },
                        Pattern {
                            kind: PatternKind::EnumConstructor {
                                enum_name: None,
                                variant: "B".to_string(),
                                args: None,
                            },
                            span: dummy_span(),
                        },
                    ]),
                    span: dummy_span(),
                },
                guard: None,
                body: dummy_expr(),
            },
            MatchArm {
                pattern: Pattern {
                    kind: PatternKind::EnumConstructor {
                        enum_name: None,
                        variant: "C".to_string(),
                        args: None,
                    },
                    span: dummy_span(),
                },
                guard: None,
                body: dummy_expr(),
            },
        ];

        let result = check_exhaustiveness(
            &arms,
            &Type::Named("ABC".to_string()),
            dummy_span(),
            &symbol_table,
        );
        assert!(result.is_exhaustive);
    }
}
