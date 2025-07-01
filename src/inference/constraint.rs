use crate::types::Type;
use crate::source::Span;

/// Represents a constraint between types that must be satisfied
#[derive(Debug, Clone, PartialEq)]
pub struct Constraint {
    pub kind: ConstraintKind,
    pub span: Span,
}

/// Different kinds of type constraints
#[derive(Debug, Clone, PartialEq)]
pub enum ConstraintKind {
    /// Two types must be equal
    Equality(Type, Type),
}

impl Constraint {
    /// Create a new equality constraint
    pub fn equality(t1: Type, t2: Type, span: Span) -> Self {
        Constraint {
            kind: ConstraintKind::Equality(t1, t2),
            span,
        }
    }
}

impl std::fmt::Display for Constraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            ConstraintKind::Equality(t1, t2) => write!(f, "{} = {}", t1, t2),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::SourceLocation;

    #[test]
    fn test_constraint_creation() {
        let start = SourceLocation::new(1, 1, 0);
        let end = SourceLocation::new(1, 10, 10);
        let span = Span::new(start, end);
        let c = Constraint::equality(
            Type::I32,
            Type::TypeVar(0),
            span,
        );
        
        match &c.kind {
            ConstraintKind::Equality(t1, t2) => {
                assert_eq!(t1, &Type::I32);
                assert_eq!(t2, &Type::TypeVar(0));
            }
        }
        
        assert_eq!(c.span, span);
    }

    #[test]
    fn test_constraint_display() {
        let span = Span::new(SourceLocation::new(1, 1, 0), SourceLocation::new(1, 10, 10));
        let c = Constraint::equality(
            Type::TypeVar(1),
            Type::Array(Box::new(Type::I32)),
            span,
        );
        
        assert_eq!(c.to_string(), "T1 = [i32]");
    }
}