use super::optimized_substitution::{optimized_occurs_check, OptimizedSubstitution};
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
                    format!("Infinite type: T{} cannot be unified with {id, ty}"),
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
                    format!("Infinite type: T{} cannot be unified with {id, ty}"),
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

        // Generic types - unify if same base name and all type arguments unify
        (Type::Generic { name: n1, args: a1 }, Type::Generic { name: n2, args: a2 }) => {
            if n1 != n2 {
                return Err(Error::new(
                    ErrorKind::TypeError,
                    format!("Generic type mismatch: {} != {n1, n2}"),
                )
                .with_location(span.start));
            }

            if a1.len() != a2.len() {
                return Err(Error::new(
                    ErrorKind::TypeError,
                    format!(
                        "Generic type argument count mismatch: {} has {} arguments, {} has {}",
                        n1,
                        a1.len(),
                        n2,
                        a2.len()
                    ),
                )
                .with_location(span.start));
            }

            let mut subst = Substitution::new();
            for (arg1, arg2) in a1.iter().zip(a2.iter()) {
                let arg_subst = unify(arg1, arg2, span)?;
                subst.compose(arg_subst);
            }
            Ok(subst)
        }

        // Tuple types - unify if all elements unify
        (Type::Tuple(elems1), Type::Tuple(elems2)) => {
            if elems1.len() != elems2.len() {
                return Err(Error::new(
                    ErrorKind::TypeError,
                    format!(
                        "Tuple size mismatch: ({}) != ({})",
                        elems1.len(),
                        elems2.len()
                    ),
                )
                .with_location(span.start));
            }

            let mut subst = Substitution::new();
            for (elem1, elem2) in elems1.iter().zip(elems2.iter()) {
                let elem_subst = unify(elem1, elem2, span)?;
                subst.compose(elem_subst);
            }
            Ok(subst)
        }

        // Reference types - unify if mutability matches and inner types unify
        (
            Type::Reference {
                mutable: m1,
                inner: i1,
            },
            Type::Reference {
                mutable: m2,
                inner: i2,
            },
        ) => {
            if m1 != m2 {
                return Err(Error::new(
                    ErrorKind::TypeError,
                    format!(
                        "Reference mutability mismatch: {} != {}",
                        if *m1 { "&mut" } else { "&" },
                        if *m2 { "&mut" } else { "&" }
                    ),
                )
                .with_location(span.start));
            }
            unify(i1, i2, span)
        }

        // Option types - unify inner types
        (Type::Option(inner1), Type::Option(inner2)) => unify(inner1, inner2, span),

        // Type mismatch
        _ => Err(Error::new(
            ErrorKind::TypeError,
            format!("Type mismatch: cannot unify {} with {t1, t2}"),
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

    #[test]
    fn test_unify_generic_types() {
        let span = test_span();

        // Same generic type with same args unifies
        let gen1 = Type::Generic {
            name: "Vec".to_string(),
            args: vec![Type::I32],
        };
        let gen2 = Type::Generic {
            name: "Vec".to_string(),
            args: vec![Type::I32],
        };
        assert!(unify(&gen1, &gen2, span).is_ok());

        // Different generic names don't unify
        let gen3 = Type::Generic {
            name: "Option".to_string(),
            args: vec![Type::I32],
        };
        assert!(unify(&gen1, &gen3, span).is_err());

        // Different arg counts don't unify
        let gen4 = Type::Generic {
            name: "Vec".to_string(),
            args: vec![Type::I32, Type::Bool],
        };
        assert!(unify(&gen1, &gen4, span).is_err());

        // Generic with type variable
        let gen5 = Type::Generic {
            name: "Vec".to_string(),
            args: vec![Type::TypeVar(0)],
        };
        let result = unify(&gen5, &gen1, span).unwrap();
        assert_eq!(result.get(0), Some(&Type::I32));
    }

    #[test]
    fn test_unify_tuple_types() {
        let span = test_span();

        // Same tuple types unify
        let tup1 = Type::Tuple(vec![Type::I32, Type::Bool]);
        let tup2 = Type::Tuple(vec![Type::I32, Type::Bool]);
        assert!(unify(&tup1, &tup2, span).is_ok());

        // Different lengths don't unify
        let tup3 = Type::Tuple(vec![Type::I32]);
        assert!(unify(&tup1, &tup3, span).is_err());

        // Different element types don't unify
        let tup4 = Type::Tuple(vec![Type::F32, Type::Bool]);
        assert!(unify(&tup1, &tup4, span).is_err());

        // Tuple with type variables
        let tup5 = Type::Tuple(vec![Type::TypeVar(0), Type::TypeVar(1)]);
        let result = unify(&tup5, &tup1, span).unwrap();
        assert_eq!(result.get(0), Some(&Type::I32));
        assert_eq!(result.get(1), Some(&Type::Bool));
    }

    #[test]
    fn test_unify_reference_types() {
        let span = test_span();

        // Same reference types unify
        let ref1 = Type::Reference {
            mutable: false,
            inner: Box::new(Type::I32),
        };
        let ref2 = Type::Reference {
            mutable: false,
            inner: Box::new(Type::I32),
        };
        assert!(unify(&ref1, &ref2, span).is_ok());

        // Different mutability doesn't unify
        let ref3 = Type::Reference {
            mutable: true,
            inner: Box::new(Type::I32),
        };
        assert!(unify(&ref1, &ref3, span).is_err());

        // Reference with type variable
        let ref4 = Type::Reference {
            mutable: false,
            inner: Box::new(Type::TypeVar(0)),
        };
        let result = unify(&ref4, &ref1, span).unwrap();
        assert_eq!(result.get(0), Some(&Type::I32));
    }

    #[test]
    fn test_unify_option_types() {
        let span = test_span();

        // Same option types unify
        let opt1 = Type::Option(Box::new(Type::I32));
        let opt2 = Type::Option(Box::new(Type::I32));
        assert!(unify(&opt1, &opt2, span).is_ok());

        // Different inner types don't unify
        let opt3 = Type::Option(Box::new(Type::Bool));
        assert!(unify(&opt1, &opt3, span).is_err());

        // Option with type variable
        let opt4 = Type::Option(Box::new(Type::TypeVar(0)));
        let result = unify(&opt4, &opt1, span).unwrap();
        assert_eq!(result.get(0), Some(&Type::I32));
    }

    #[test]
    fn test_unify_nested_generics() {
        let span = test_span();

        // Box<Option<i32>>
        let nested1 = Type::Generic {
            name: "Box".to_string(),
            args: vec![Type::Option(Box::new(Type::I32))],
        };

        // Box<Option<T0>>
        let nested2 = Type::Generic {
            name: "Box".to_string(),
            args: vec![Type::Option(Box::new(Type::TypeVar(0)))],
        };

        let result = unify(&nested2, &nested1, span).unwrap();
        assert_eq!(result.get(0), Some(&Type::I32));
    }
}

/// Optimized unify function that returns OptimizedSubstitution for better performance
pub fn unify_optimized(t1: &Type, t2: &Type, span: Span) -> Result<OptimizedSubstitution, Error> {
    match (t1, t2) {
        // Two type variables
        (Type::TypeVar(id1), Type::TypeVar(id2)) if id1 == id2 => Ok(OptimizedSubstitution::new()),

        // Type variable on the left
        (Type::TypeVar(id), ty) => {
            if optimized_occurs_check(*id, ty) {
                Err(Error::new(
                    ErrorKind::TypeError,
                    format!("Infinite type: T{} cannot be unified with {id, ty}"),
                )
                .with_location(span.start))
            } else {
                Ok(OptimizedSubstitution::singleton(*id, ty.clone()))
            }
        }

        // Type variable on the right
        (ty, Type::TypeVar(id)) => {
            if optimized_occurs_check(*id, ty) {
                Err(Error::new(
                    ErrorKind::TypeError,
                    format!("Infinite type: T{} cannot be unified with {id, ty}"),
                )
                .with_location(span.start))
            } else {
                Ok(OptimizedSubstitution::singleton(*id, ty.clone()))
            }
        }

        // Unknown type (gradual typing) - unifies with anything
        (Type::Unknown, _) | (_, Type::Unknown) => Ok(OptimizedSubstitution::new()),

        // Basic types must match exactly
        (Type::I32, Type::I32)
        | (Type::F32, Type::F32)
        | (Type::Bool, Type::Bool)
        | (Type::String, Type::String) => Ok(OptimizedSubstitution::new()),

        // Array types - unify element types
        (Type::Array(elem1), Type::Array(elem2)) => unify_optimized(elem1, elem2, span),

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

            let mut subst = OptimizedSubstitution::new();

            // Unify each parameter
            for (param1, param2) in p1.iter().zip(p2.iter()) {
                let param_subst = unify_optimized(param1, param2, span)?;
                subst.compose(param_subst);
            }

            // Unify return types
            let ret_subst = unify_optimized(r1, r2, span)?;
            subst.compose(ret_subst);

            Ok(subst)
        }

        // All other cases are unification failures
        _ => Err(Error::new(
            ErrorKind::TypeError,
            format!("Cannot unify {} with {t1, t2}"),
        )
        .with_location(span.start)),
    }
}
