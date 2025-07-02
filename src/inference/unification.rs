use super::substitution::{occurs_check, Substitution};
use crate::error::{Error, ErrorKind};
use crate::source::Span;
use crate::types::Type;

/// Unify two types, returning a substitution that makes them equal
pub fn unify(t1: &Type, t2: &Type, span: Span) -> Result<Substitution, Error> {
    match (t1, t2) {
        // Two type variables
        (Type::TypeVar(id1), Type::TypeVar(id2)) if id1 == id2 => Ok(Substitution::new()),

        // Type variable on the left
        (Type::TypeVar(id), ty) => {
            if occurs_check(*id, ty) {
                Err(Error::new(
                    ErrorKind::TypeError,
                    format!("Infinite type: T{} cannot be unified with {}", id, ty),
                )
                .with_location(span.start))
            } else {
                Ok(Substitution::singleton(*id, ty.clone()))
            }
        }

        // Type variable on the right
        (ty, Type::TypeVar(id)) => {
            if occurs_check(*id, ty) {
                Err(Error::new(
                    ErrorKind::TypeError,
                    format!("Infinite type: T{} cannot be unified with {}", id, ty),
                )
                .with_location(span.start))
            } else {
                Ok(Substitution::singleton(*id, ty.clone()))
            }
        }

        // Unknown type (gradual typing) - unifies with anything
        (Type::Unknown, _) | (_, Type::Unknown) => Ok(Substitution::new()),

        // Basic types must match exactly
        (Type::I32, Type::I32)
        | (Type::F32, Type::F32)
        | (Type::Bool, Type::Bool)
        | (Type::String, Type::String) => Ok(Substitution::new()),

        // Array types - unify element types
        (Type::Array(elem1), Type::Array(elem2)) => unify(elem1, elem2, span),

        // Function types - unify parameters and return type
        (
            Type::Function {
                params: p1,
                ret: r1,
            },
            Type::Function {
                params: p2,
                ret: r2,
            },
        ) => {
            if p1.len() != p2.len() {
                return Err(Error::new(
                    ErrorKind::TypeError,
                    format!(
                        "Function type mismatch: expected {} parameters, found {}",
                        p1.len(),
                        p2.len()
                    ),
                )
                .with_location(span.start));
            }

            let mut subst = Substitution::new();

            // Unify each parameter
            for (param1, param2) in p1.iter().zip(p2.iter()) {
                let param_subst = unify(param1, param2, span)?;
                subst.compose(param_subst);
            }

            // Unify return types
            let ret_subst = unify(r1, r2, span)?;
            subst.compose(ret_subst);

            Ok(subst)
        }

        // Result types - unify ok and err types
        (Type::Result { ok: o1, err: e1 }, Type::Result { ok: o2, err: e2 }) => {
            let mut subst = unify(o1, o2, span)?;
            let err_subst = unify(e1, e2, span)?;
            subst.compose(err_subst);
            Ok(subst)
        }

        // Named types must have the same name
        (Type::Named(n1), Type::Named(n2)) if n1 == n2 => Ok(Substitution::new()),

        // Type mismatch
        _ => Err(Error::new(
            ErrorKind::TypeError,
            format!("Type mismatch: cannot unify {} with {}", t1, t2),
        )
        .with_location(span.start)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::SourceLocation;

    fn test_span() -> Span {
        Span::new(SourceLocation::new(1, 1, 0), SourceLocation::new(1, 10, 10))
    }

    #[test]
    fn test_unify_basic_types() {
        let span = test_span();

        // Same basic types unify
        assert!(unify(&Type::I32, &Type::I32, span).is_ok());
        assert!(unify(&Type::Bool, &Type::Bool, span).is_ok());

        // Different basic types don't unify
        assert!(unify(&Type::I32, &Type::F32, span).is_err());
        assert!(unify(&Type::Bool, &Type::String, span).is_err());
    }

    #[test]
    fn test_unify_type_variables() {
        let span = test_span();

        // Same type variable unifies with itself
        let result = unify(&Type::TypeVar(0), &Type::TypeVar(0), span).unwrap();
        assert!(result.is_empty());

        // Type variable unifies with concrete type
        let result = unify(&Type::TypeVar(0), &Type::I32, span).unwrap();
        assert_eq!(result.get(0), Some(&Type::I32));

        // Type variable unifies with another type variable
        let result = unify(&Type::TypeVar(0), &Type::TypeVar(1), span).unwrap();
        assert_eq!(result.get(0), Some(&Type::TypeVar(1)));
    }

    #[test]
    fn test_unify_unknown_type() {
        let span = test_span();

        // Unknown unifies with anything
        assert!(unify(&Type::Unknown, &Type::I32, span).is_ok());
        assert!(unify(&Type::String, &Type::Unknown, span).is_ok());
        assert!(unify(&Type::Unknown, &Type::Unknown, span).is_ok());

        let array_ty = Type::Array(Box::new(Type::I32));
        assert!(unify(&Type::Unknown, &array_ty, span).is_ok());
    }

    #[test]
    fn test_unify_array_types() {
        let span = test_span();

        // Arrays with same element type unify
        let arr1 = Type::Array(Box::new(Type::I32));
        let arr2 = Type::Array(Box::new(Type::I32));
        assert!(unify(&arr1, &arr2, span).is_ok());

        // Arrays with different element types don't unify
        let arr3 = Type::Array(Box::new(Type::Bool));
        assert!(unify(&arr1, &arr3, span).is_err());

        // Array with type variable element
        let arr4 = Type::Array(Box::new(Type::TypeVar(0)));
        let result = unify(&arr4, &arr1, span).unwrap();
        assert_eq!(result.get(0), Some(&Type::I32));
    }

    #[test]
    fn test_unify_function_types() {
        let span = test_span();

        // Functions with same signature unify
        let fn1 = Type::Function {
            params: vec![Type::I32, Type::Bool],
            ret: Box::new(Type::String),
        };
        let fn2 = Type::Function {
            params: vec![Type::I32, Type::Bool],
            ret: Box::new(Type::String),
        };
        assert!(unify(&fn1, &fn2, span).is_ok());

        // Functions with different parameter count don't unify
        let fn3 = Type::Function {
            params: vec![Type::I32],
            ret: Box::new(Type::String),
        };
        assert!(unify(&fn1, &fn3, span).is_err());

        // Functions with type variables
        let fn4 = Type::Function {
            params: vec![Type::TypeVar(0), Type::Bool],
            ret: Box::new(Type::TypeVar(1)),
        };
        let fn5 = Type::Function {
            params: vec![Type::I32, Type::Bool],
            ret: Box::new(Type::String),
        };
        let result = unify(&fn4, &fn5, span).unwrap();
        assert_eq!(result.get(0), Some(&Type::I32));
        assert_eq!(result.get(1), Some(&Type::String));
    }

    #[test]
    fn test_occurs_check_prevents_infinite_types() {
        let span = test_span();

        // T0 = [T0] would create an infinite type
        let infinite_array = Type::Array(Box::new(Type::TypeVar(0)));
        let result = unify(&Type::TypeVar(0), &infinite_array, span);
        assert!(result.is_err());

        // T0 = (T0) -> T1 would create an infinite type
        let infinite_fn = Type::Function {
            params: vec![Type::TypeVar(0)],
            ret: Box::new(Type::TypeVar(1)),
        };
        let result = unify(&Type::TypeVar(0), &infinite_fn, span);
        assert!(result.is_err());
    }
}
