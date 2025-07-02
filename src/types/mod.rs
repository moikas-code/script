use std::collections::HashMap;
use std::fmt;

pub mod conversion;
pub mod generics;

/// The main type representation in the Script language
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    /// 32-bit signed integer
    I32,
    /// 32-bit floating point
    F32,
    /// Boolean type
    Bool,
    /// String type
    String,
    /// Unknown type for gradual typing
    Unknown,
    /// Array type with element type
    Array(Box<Type>),
    /// Function type with parameter types and return type
    Function { params: Vec<Type>, ret: Box<Type> },
    /// Result type for error handling
    Result { ok: Box<Type>, err: Box<Type> },
    /// Future type for async operations
    Future(Box<Type>),
    /// Named type for future extensions (e.g., structs, actors)
    Named(String),
    /// Type variable for type inference
    TypeVar(u32),
    /// Option type for nullable values
    Option(Box<Type>),
    /// Never type for functions that never return
    Never,
    /// Generic type with type parameters (e.g., Vec<T>, Map<K, V>)
    Generic { name: String, args: Vec<Type> },
    /// Type parameter in generic context (named type variable)
    TypeParam(String),
}

impl Type {
    /// Check if two types are equal
    pub fn equals(&self, other: &Type) -> bool {
        match (self, other) {
            // Unknown type is compatible with any type (gradual typing)
            (Type::Unknown, _) | (_, Type::Unknown) => true,

            // Basic types must match exactly
            (Type::I32, Type::I32) => true,
            (Type::F32, Type::F32) => true,
            (Type::Bool, Type::Bool) => true,
            (Type::String, Type::String) => true,

            // Array types must have matching element types
            (Type::Array(a), Type::Array(b)) => a.equals(b),

            // Function types must have matching signatures
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
                p1.len() == p2.len()
                    && p1.iter().zip(p2.iter()).all(|(a, b)| a.equals(b))
                    && r1.equals(r2)
            }

            // Result types must have matching ok and err types
            (Type::Result { ok: o1, err: e1 }, Type::Result { ok: o2, err: e2 }) => {
                o1.equals(o2) && e1.equals(e2)
            }

            // Future types must have matching inner types
            (Type::Future(a), Type::Future(b)) => a.equals(b),

            // Named types must have the same name
            (Type::Named(n1), Type::Named(n2)) => n1 == n2,

            // Type variables are equal if they have the same ID
            (Type::TypeVar(id1), Type::TypeVar(id2)) => id1 == id2,

            // Option types must have matching inner types
            (Type::Option(a), Type::Option(b)) => a.equals(b),

            // Never types are always equal
            (Type::Never, Type::Never) => true,

            // Generic types must have same name and matching type arguments
            (Type::Generic { name: n1, args: a1 }, Type::Generic { name: n2, args: a2 }) => {
                n1 == n2
                    && a1.len() == a2.len()
                    && a1.iter().zip(a2.iter()).all(|(t1, t2)| t1.equals(t2))
            }

            // Type parameters are equal if they have the same name
            (Type::TypeParam(n1), Type::TypeParam(n2)) => n1 == n2,

            // All other combinations are not equal
            _ => false,
        }
    }

    /// Check if this type is assignable to another type
    /// This is more permissive than equality for gradual typing
    pub fn is_assignable_to(&self, target: &Type) -> bool {
        match (self, target) {
            // Unknown type can be assigned to/from any type
            (Type::Unknown, _) | (_, Type::Unknown) => true,

            // Same types are assignable
            (a, b) if a.equals(b) => true,

            // Numeric conversions could be allowed here in the future
            // For now, require exact matches
            _ => false,
        }
    }

    /// Check if this type is a numeric type
    pub fn is_numeric(&self) -> bool {
        matches!(self, Type::I32 | Type::F32)
    }

    /// Check if this type is comparable (for comparison operators)
    pub fn is_comparable(&self) -> bool {
        matches!(self, Type::I32 | Type::F32 | Type::Bool | Type::String)
    }

    /// Get the return type of a function type
    pub fn return_type(&self) -> Option<&Type> {
        match self {
            Type::Function { ret, .. } => Some(ret),
            _ => None,
        }
    }

    /// Get the parameter types of a function type
    pub fn param_types(&self) -> Option<&[Type]> {
        match self {
            Type::Function { params, .. } => Some(params),
            _ => None,
        }
    }

    /// Get the element type of an array
    pub fn element_type(&self) -> Option<&Type> {
        match self {
            Type::Array(elem) => Some(elem),
            _ => None,
        }
    }

    /// Get the inner type of a Future
    pub fn future_type(&self) -> Option<&Type> {
        match self {
            Type::Future(inner) => Some(inner),
            _ => None,
        }
    }

    /// Check if this type is a Future
    pub fn is_future(&self) -> bool {
        matches!(self, Type::Future(_))
    }

    /// Check if this type is Unknown
    pub fn is_unknown(&self) -> bool {
        matches!(self, Type::Unknown)
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::I32 => write!(f, "i32"),
            Type::F32 => write!(f, "f32"),
            Type::Bool => write!(f, "bool"),
            Type::String => write!(f, "string"),
            Type::Unknown => write!(f, "unknown"),
            Type::Array(elem) => write!(f, "[{}]", elem),
            Type::Function { params, ret } => {
                write!(f, "(")?;
                for (i, param) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", param)?;
                }
                write!(f, ") -> {}", ret)
            }
            Type::Result { ok, err } => write!(f, "Result<{}, {}>", ok, err),
            Type::Future(inner) => write!(f, "Future<{}>", inner),
            Type::Named(name) => write!(f, "{}", name),
            Type::TypeVar(id) => write!(f, "T{}", id),
            Type::Option(inner) => write!(f, "Option<{}>", inner),
            Type::Never => write!(f, "never"),
            Type::Generic { name, args } => {
                write!(f, "{}", name)?;
                if !args.is_empty() {
                    write!(f, "<")?;
                    for (i, arg) in args.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", arg)?;
                    }
                    write!(f, ">")?;
                }
                Ok(())
            }
            Type::TypeParam(name) => write!(f, "{}", name),
        }
    }
}

/// Type environment for managing type information
#[derive(Debug, Clone)]
pub struct TypeEnv {
    /// Stack of scopes, each scope is a HashMap of variable names to types
    scopes: Vec<HashMap<String, Type>>,
    /// Global type definitions (for named types)
    type_defs: HashMap<String, Type>,
}

impl TypeEnv {
    /// Create a new empty type environment
    pub fn new() -> Self {
        TypeEnv {
            scopes: vec![HashMap::new()],
            type_defs: HashMap::new(),
        }
    }

    /// Enter a new scope
    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// Exit the current scope
    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    /// Define a variable in the current scope
    pub fn define(&mut self, name: String, ty: Type) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, ty);
        }
    }

    /// Look up a variable's type
    pub fn lookup(&self, name: &str) -> Option<&Type> {
        // Search from innermost to outermost scope
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Some(ty);
            }
        }
        None
    }

    /// Define a named type
    pub fn define_type(&mut self, name: String, ty: Type) {
        self.type_defs.insert(name, ty);
    }

    /// Look up a named type
    pub fn lookup_type(&self, name: &str) -> Option<&Type> {
        self.type_defs.get(name)
    }

    /// Get the number of scopes
    pub fn scope_depth(&self) -> usize {
        self.scopes.len()
    }
}

impl Default for TypeEnv {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_equality() {
        assert!(Type::I32.equals(&Type::I32));
        assert!(Type::F32.equals(&Type::F32));
        assert!(Type::Bool.equals(&Type::Bool));
        assert!(Type::String.equals(&Type::String));

        assert!(!Type::I32.equals(&Type::F32));
        assert!(!Type::Bool.equals(&Type::String));
    }

    #[test]
    fn test_unknown_type_compatibility() {
        assert!(Type::Unknown.equals(&Type::I32));
        assert!(Type::F32.equals(&Type::Unknown));
        assert!(Type::Unknown.equals(&Type::Unknown));
    }

    #[test]
    fn test_array_type_equality() {
        let int_array = Type::Array(Box::new(Type::I32));
        let float_array = Type::Array(Box::new(Type::F32));
        let int_array2 = Type::Array(Box::new(Type::I32));

        assert!(int_array.equals(&int_array2));
        assert!(!int_array.equals(&float_array));
    }

    #[test]
    fn test_function_type_equality() {
        let fn1 = Type::Function {
            params: vec![Type::I32, Type::I32],
            ret: Box::new(Type::I32),
        };
        let fn2 = Type::Function {
            params: vec![Type::I32, Type::I32],
            ret: Box::new(Type::I32),
        };
        let fn3 = Type::Function {
            params: vec![Type::I32],
            ret: Box::new(Type::I32),
        };
        let fn4 = Type::Function {
            params: vec![Type::I32, Type::I32],
            ret: Box::new(Type::F32),
        };

        assert!(fn1.equals(&fn2));
        assert!(!fn1.equals(&fn3)); // Different param count
        assert!(!fn1.equals(&fn4)); // Different return type
    }

    #[test]
    fn test_result_type_equality() {
        let result1 = Type::Result {
            ok: Box::new(Type::I32),
            err: Box::new(Type::String),
        };
        let result2 = Type::Result {
            ok: Box::new(Type::I32),
            err: Box::new(Type::String),
        };
        let result3 = Type::Result {
            ok: Box::new(Type::F32),
            err: Box::new(Type::String),
        };

        assert!(result1.equals(&result2));
        assert!(!result1.equals(&result3));
    }

    #[test]
    fn test_future_type_equality() {
        let future1 = Type::Future(Box::new(Type::I32));
        let future2 = Type::Future(Box::new(Type::I32));
        let future3 = Type::Future(Box::new(Type::String));

        assert!(future1.equals(&future2));
        assert!(!future1.equals(&future3));

        // Test is_future helper
        assert!(future1.is_future());
        assert!(!Type::I32.is_future());

        // Test future_type helper
        assert_eq!(future1.future_type(), Some(&Type::I32));
        assert_eq!(Type::I32.future_type(), None);
    }

    #[test]
    fn test_type_display() {
        assert_eq!(Type::I32.to_string(), "i32");
        assert_eq!(Type::F32.to_string(), "f32");
        assert_eq!(Type::Bool.to_string(), "bool");
        assert_eq!(Type::String.to_string(), "string");
        assert_eq!(Type::Unknown.to_string(), "unknown");

        let array_type = Type::Array(Box::new(Type::I32));
        assert_eq!(array_type.to_string(), "[i32]");

        let fn_type = Type::Function {
            params: vec![Type::I32, Type::Bool],
            ret: Box::new(Type::String),
        };
        assert_eq!(fn_type.to_string(), "(i32, bool) -> string");

        let result_type = Type::Result {
            ok: Box::new(Type::I32),
            err: Box::new(Type::String),
        };
        assert_eq!(result_type.to_string(), "Result<i32, string>");

        let future_type = Type::Future(Box::new(Type::String));
        assert_eq!(future_type.to_string(), "Future<string>");
    }

    #[test]
    fn test_type_env_basic() {
        let mut env = TypeEnv::new();

        // Define a variable
        env.define("x".to_string(), Type::I32);
        assert_eq!(env.lookup("x"), Some(&Type::I32));
        assert_eq!(env.lookup("y"), None);
    }

    #[test]
    fn test_type_env_scoping() {
        let mut env = TypeEnv::new();

        // Define in outer scope
        env.define("x".to_string(), Type::I32);

        // Enter new scope
        env.push_scope();
        assert_eq!(env.scope_depth(), 2);

        // Can still see outer scope
        assert_eq!(env.lookup("x"), Some(&Type::I32));

        // Shadow in inner scope
        env.define("x".to_string(), Type::F32);
        assert_eq!(env.lookup("x"), Some(&Type::F32));

        // Define new var in inner scope
        env.define("y".to_string(), Type::Bool);

        // Exit inner scope
        env.pop_scope();
        assert_eq!(env.scope_depth(), 1);

        // Back to outer scope value
        assert_eq!(env.lookup("x"), Some(&Type::I32));
        // Inner scope var is gone
        assert_eq!(env.lookup("y"), None);
    }

    #[test]
    fn test_type_env_named_types() {
        let mut env = TypeEnv::new();

        // Define a named type
        let my_result = Type::Result {
            ok: Box::new(Type::I32),
            err: Box::new(Type::String),
        };
        env.define_type("MyResult".to_string(), my_result.clone());

        // Look it up
        assert_eq!(env.lookup_type("MyResult"), Some(&my_result));
        assert_eq!(env.lookup_type("Unknown"), None);
    }

    #[test]
    fn test_type_helpers() {
        // Test numeric check
        assert!(Type::I32.is_numeric());
        assert!(Type::F32.is_numeric());
        assert!(!Type::Bool.is_numeric());
        assert!(!Type::String.is_numeric());

        // Test comparable check
        assert!(Type::I32.is_comparable());
        assert!(Type::F32.is_comparable());
        assert!(Type::Bool.is_comparable());
        assert!(Type::String.is_comparable());
        assert!(!Type::Array(Box::new(Type::I32)).is_comparable());

        // Test function type helpers
        let fn_type = Type::Function {
            params: vec![Type::I32, Type::Bool],
            ret: Box::new(Type::String),
        };
        assert_eq!(fn_type.return_type(), Some(&Type::String));
        assert_eq!(
            fn_type.param_types(),
            Some(&vec![Type::I32, Type::Bool][..])
        );
        assert_eq!(Type::I32.return_type(), None);

        // Test array element type
        let array_type = Type::Array(Box::new(Type::F32));
        assert_eq!(array_type.element_type(), Some(&Type::F32));
        assert_eq!(Type::I32.element_type(), None);
    }

    #[test]
    fn test_is_unknown() {
        // Test Unknown type
        assert!(Type::Unknown.is_unknown());

        // Test non-Unknown types
        assert!(!Type::I32.is_unknown());
        assert!(!Type::F32.is_unknown());
        assert!(!Type::Bool.is_unknown());
        assert!(!Type::String.is_unknown());
        assert!(!Type::Array(Box::new(Type::I32)).is_unknown());
        assert!(!Type::Future(Box::new(Type::I32)).is_unknown());
        assert!(!Type::Never.is_unknown());
    }

    #[test]
    fn test_assignability() {
        // Basic assignability
        assert!(Type::I32.is_assignable_to(&Type::I32));
        assert!(!Type::I32.is_assignable_to(&Type::F32));

        // Unknown type assignability
        assert!(Type::Unknown.is_assignable_to(&Type::I32));
        assert!(Type::I32.is_assignable_to(&Type::Unknown));

        // Complex type assignability
        let array1 = Type::Array(Box::new(Type::I32));
        let array2 = Type::Array(Box::new(Type::I32));
        let array3 = Type::Array(Box::new(Type::F32));

        assert!(array1.is_assignable_to(&array2));
        assert!(!array1.is_assignable_to(&array3));
    }
}
