//! Pattern exhaustiveness checking for match expressions
//!
//! This module implements an algorithm to verify that match expressions
//! cover all possible cases, ensuring no runtime panics from missing patterns.

use crate::parser::{Literal, MatchArm, Pattern, PatternKind};
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
    scrutinee_span: Span,
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
    let is_exhaustive = has_wildcard || check_simple_exhaustiveness(arms, scrutinee_type);

    // Find redundant patterns
    let redundant_patterns = find_redundant_patterns(arms);

    // Generate missing patterns
    let missing_patterns = if !is_exhaustive {
        generate_missing_patterns(arms, scrutinee_type)
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
fn check_simple_exhaustiveness(arms: &[MatchArm], scrutinee_type: &Type) -> bool {
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
        Type::Named(name) => {
            // For custom types, we'd need to check against enum variants
            // For now, require a wildcard
            let _ = name;
            false
        }
        _ => false,
    }
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
fn generate_missing_patterns(arms: &[MatchArm], scrutinee_type: &Type) -> Vec<String> {
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
        Span::default()
    }

    fn dummy_expr() -> Expr {
        Expr {
            kind: ExprKind::Literal(Literal::Number(0.0)),
            span: dummy_span(),
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

        let result = check_exhaustiveness(&arms, &Type::Bool, dummy_span());
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

        let result = check_exhaustiveness(&arms, &Type::Bool, dummy_span());
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

        let result = check_exhaustiveness(&arms, &Type::I32, dummy_span());
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

        let result = check_exhaustiveness(&arms, &Type::I32, dummy_span());
        assert!(result.is_exhaustive);
        assert_eq!(result.redundant_patterns.len(), 1);
        assert_eq!(result.redundant_patterns[0].0, 1); // Second pattern is redundant
    }
}
