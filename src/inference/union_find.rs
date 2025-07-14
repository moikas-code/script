use crate::types::Type;
use std::collections::HashMap;

/// Union-Find data structure optimized for type unification
/// Provides nearly O(n·α(n)) complexity where α is the inverse Ackermann function
#[derive(Debug, Clone)]
pub struct UnionFind {
    /// Parent pointers for union-find with path compression
    parent: HashMap<u32, u32>,
    /// Rank for union by rank optimization
    rank: HashMap<u32, u32>,
    /// Representative type for each equivalence class
    representatives: HashMap<u32, Type>,
    /// Next available type variable ID
    next_var_id: u32,
}

impl UnionFind {
    /// Create a new union-find structure
    pub fn new() -> Self {
        UnionFind {
            parent: HashMap::new(),
            rank: HashMap::new(),
            representatives: HashMap::new(),
            next_var_id: 0,
        }
    }

    /// Generate a fresh type variable
    pub fn fresh_type_var(&mut self) -> Type {
        let var_id = self.next_var_id;
        self.next_var_id = self.next_var_id.wrapping_add(1);

        // Initialize as its own parent with rank 0
        self.parent.insert(var_id, var_id);
        self.rank.insert(var_id, 0);
        self.representatives.insert(var_id, Type::TypeVar(var_id));

        Type::TypeVar(var_id)
    }

    /// Find the root representative of a type variable with path compression
    fn find_root(&mut self, var_id: u32) -> u32 {
        if !self.parent.contains_key(&var_id) {
            // Initialize if not present
            self.parent.insert(var_id, var_id);
            self.rank.insert(var_id, 0);
            self.representatives.insert(var_id, Type::TypeVar(var_id));
            return var_id;
        }

        let parent = self.parent[&var_id];
        if parent != var_id {
            // Path compression: make parent point directly to root
            let root = self.find_root(parent);
            self.parent.insert(var_id, root);
            root
        } else {
            var_id
        }
    }

    /// Union two type variables using union by rank
    pub fn union(&mut self, var1: u32, var2: u32, concrete_type: Option<Type>) -> bool {
        let root1 = self.find_root(var1);
        let root2 = self.find_root(var2);

        if root1 == root2 {
            return true; // Already unified
        }

        let rank1 = self.rank[&root1];
        let rank2 = self.rank[&root2];

        // Union by rank to keep tree shallow
        let (new_root, old_root) = if rank1 > rank2 {
            (root1, root2)
        } else if rank1 < rank2 {
            (root2, root1)
        } else {
            // Equal ranks, choose arbitrarily and increment rank
            self.rank.insert(root1, rank1 + 1);
            (root1, root2)
        };

        // Update parent pointer
        self.parent.insert(old_root, new_root);

        // Update representative type
        if let Some(ty) = concrete_type {
            self.representatives.insert(new_root, ty);
        } else {
            // Keep the representative of the new root
            let root_type = self.representatives[&new_root].clone();
            self.representatives.insert(new_root, root_type);
        }

        true
    }

    /// Unify a type variable with a concrete type
    pub fn unify_with_concrete(&mut self, var_id: u32, concrete_type: Type) -> Result<(), String> {
        // Check for occurs check
        if self.occurs_check(var_id, &concrete_type) {
            return Err(format!(
                "Infinite type: T{} occurs in {}",
                var_id, concrete_type
            ));
        }

        let root = self.find_root(var_id);

        // Check if root already has a concrete type
        if let Some(existing_type) = self.representatives.get(&root) {
            if !matches!(existing_type, Type::TypeVar(_)) {
                // Already has concrete type, check compatibility
                if existing_type != &concrete_type {
                    return Err(format!(
                        "Type conflict: cannot unify {} with {}",
                        existing_type, concrete_type
                    ));
                }
                return Ok(());
            }
        }

        // Set concrete type as representative
        self.representatives.insert(root, concrete_type);
        Ok(())
    }

    /// Unify two types using the union-find structure
    pub fn unify_types(&mut self, t1: &Type, t2: &Type) -> Result<(), String> {
        match (t1, t2) {
            // Both type variables
            (Type::TypeVar(id1), Type::TypeVar(id2)) => {
                self.union(*id1, *id2, None);
                Ok(())
            }

            // Type variable with concrete type
            (Type::TypeVar(id), concrete) | (concrete, Type::TypeVar(id)) => {
                self.unify_with_concrete(*id, concrete.clone())
            }

            // Array types
            (Type::Array(elem1), Type::Array(elem2)) => self.unify_types(elem1, elem2),

            // Function types
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
                    return Err(format!(
                        "Function arity mismatch: {} vs {}",
                        p1.len(),
                        p2.len()
                    ));
                }

                // Unify parameters
                for (param1, param2) in p1.iter().zip(p2.iter()) {
                    self.unify_types(param1, param2)?;
                }

                // Unify return types
                self.unify_types(r1, r2)
            }

            // Generic types
            (Type::Generic { name: n1, args: a1 }, Type::Generic { name: n2, args: a2 }) => {
                if n1 != n2 {
                    return Err(format!("Generic type mismatch: {} vs {}", n1, n2));
                }
                if a1.len() != a2.len() {
                    return Err(format!(
                        "Generic arity mismatch: {} vs {}",
                        a1.len(),
                        a2.len()
                    ));
                }

                for (arg1, arg2) in a1.iter().zip(a2.iter()) {
                    self.unify_types(arg1, arg2)?;
                }
                Ok(())
            }

            // Tuple types
            (Type::Tuple(types1), Type::Tuple(types2)) => {
                if types1.len() != types2.len() {
                    return Err(format!(
                        "Tuple size mismatch: {} vs {}",
                        types1.len(),
                        types2.len()
                    ));
                }

                for (t1, t2) in types1.iter().zip(types2.iter()) {
                    self.unify_types(t1, t2)?;
                }
                Ok(())
            }

            // Reference types
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
                    return Err("Reference mutability mismatch".to_string());
                }
                self.unify_types(i1, i2)
            }

            // Option types
            (Type::Option(inner1), Type::Option(inner2)) => self.unify_types(inner1, inner2),

            // Result types
            (Type::Result { ok: o1, err: e1 }, Type::Result { ok: o2, err: e2 }) => {
                self.unify_types(o1, o2)?;
                self.unify_types(e1, e2)
            }

            // Future types
            (Type::Future(inner1), Type::Future(inner2)) => self.unify_types(inner1, inner2),

            // Unknown type unifies with anything
            (Type::Unknown, _) | (_, Type::Unknown) => Ok(()),

            // Same concrete types
            (t1, t2) if t1 == t2 => Ok(()),

            // Type mismatch
            _ => Err(format!("Cannot unify {} with {}", t1, t2)),
        }
    }

    /// Get the resolved type for a type variable
    pub fn resolve_type(&mut self, ty: &Type) -> Type {
        match ty {
            Type::TypeVar(id) => {
                let root = self.find_root(*id);
                if let Some(representative) = self.representatives.get(&root).cloned() {
                    if matches!(representative, Type::TypeVar(_)) && representative != *ty {
                        // Recursively resolve
                        self.resolve_type(&representative)
                    } else {
                        representative
                    }
                } else {
                    ty.clone()
                }
            }
            Type::Array(elem) => Type::Array(Box::new(self.resolve_type(elem))),
            Type::Function { params, ret } => Type::Function {
                params: params.iter().map(|p| self.resolve_type(p)).collect(),
                ret: Box::new(self.resolve_type(ret)),
            },
            Type::Generic { name, args } => Type::Generic {
                name: name.clone(),
                args: args.iter().map(|arg| self.resolve_type(arg)).collect(),
            },
            Type::Tuple(types) => Type::Tuple(types.iter().map(|t| self.resolve_type(t)).collect()),
            Type::Reference { mutable, inner } => Type::Reference {
                mutable: *mutable,
                inner: Box::new(self.resolve_type(inner)),
            },
            Type::Option(inner) => Type::Option(Box::new(self.resolve_type(inner))),
            Type::Result { ok, err } => Type::Result {
                ok: Box::new(self.resolve_type(ok)),
                err: Box::new(self.resolve_type(err)),
            },
            Type::Future(inner) => Type::Future(Box::new(self.resolve_type(inner))),
            _ => ty.clone(),
        }
    }

    /// Check if a type variable occurs in a type (occurs check)
    fn occurs_check(&mut self, var_id: u32, ty: &Type) -> bool {
        match ty {
            Type::TypeVar(id) => {
                let root = self.find_root(*id);
                let target_root = self.find_root(var_id);
                root == target_root
            }
            Type::Array(elem) => self.occurs_check(var_id, elem),
            Type::Function { params, ret } => {
                params.iter().any(|p| self.occurs_check(var_id, p))
                    || self.occurs_check(var_id, ret)
            }
            Type::Generic { args, .. } => args.iter().any(|arg| self.occurs_check(var_id, arg)),
            Type::Tuple(types) => types.iter().any(|t| self.occurs_check(var_id, t)),
            Type::Reference { inner, .. } => self.occurs_check(var_id, inner),
            Type::Option(inner) => self.occurs_check(var_id, inner),
            Type::Result { ok, err } => {
                self.occurs_check(var_id, ok) || self.occurs_check(var_id, err)
            }
            Type::Future(inner) => self.occurs_check(var_id, inner),
            _ => false,
        }
    }

    /// Get statistics about the union-find structure
    pub fn stats(&self) -> UnionFindStats {
        UnionFindStats {
            total_variables: self.parent.len(),
            equivalence_classes: self.representatives.len(),
            max_rank: self.rank.values().max().copied().unwrap_or(0),
        }
    }
}

impl Default for UnionFind {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about the union-find structure performance
#[derive(Debug, Clone)]
pub struct UnionFindStats {
    pub total_variables: usize,
    pub equivalence_classes: usize,
    pub max_rank: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_union_find_basic() {
        let mut uf = UnionFind::new();

        let t1 = uf.fresh_type_var();
        let t2 = uf.fresh_type_var();

        if let (Type::TypeVar(id1), Type::TypeVar(id2)) = (t1, t2) {
            // Initially different roots
            assert_ne!(uf.find_root(id1), uf.find_root(id2));

            // Union them
            uf.union(id1, id2, None);

            // Now same root
            assert_eq!(uf.find_root(id1), uf.find_root(id2));
        }
    }

    #[test]
    fn test_unify_with_concrete() {
        let mut uf = UnionFind::new();

        let var = uf.fresh_type_var();
        if let Type::TypeVar(id) = var {
            uf.unify_with_concrete(id, Type::I32).unwrap();

            let resolved = uf.resolve_type(&Type::TypeVar(id));
            assert_eq!(resolved, Type::I32);
        }
    }

    #[test]
    fn test_occurs_check() {
        let mut uf = UnionFind::new();

        let var = uf.fresh_type_var();
        if let Type::TypeVar(id) = var {
            let recursive_type = Type::Array(Box::new(Type::TypeVar(id)));

            let result = uf.unify_with_concrete(id, recursive_type);
            assert!(result.is_err());
            assert!(result.unwrap_err().contains("Infinite type"));
        }
    }

    #[test]
    fn test_function_unification() {
        let mut uf = UnionFind::new();

        let var1 = uf.fresh_type_var();
        let var2 = uf.fresh_type_var();

        let fn1 = Type::Function {
            params: vec![var1.clone(), Type::I32],
            ret: Box::new(Type::String),
        };

        let fn2 = Type::Function {
            params: vec![Type::Bool, var2.clone()],
            ret: Box::new(Type::String),
        };

        uf.unify_types(&fn1, &fn2).unwrap();

        // var1 should resolve to Bool, var2 should resolve to I32
        assert_eq!(uf.resolve_type(&var1), Type::Bool);
        assert_eq!(uf.resolve_type(&var2), Type::I32);
    }

    #[test]
    fn test_path_compression() {
        let mut uf = UnionFind::new();

        // Create a chain: v1 -> v2 -> v3 -> v4
        let v1 = uf.fresh_type_var();
        let v2 = uf.fresh_type_var();
        let v3 = uf.fresh_type_var();
        let v4 = uf.fresh_type_var();

        if let (Type::TypeVar(id1), Type::TypeVar(id2), Type::TypeVar(id3), Type::TypeVar(id4)) =
            (v1, v2, v3, v4)
        {
            uf.union(id1, id2, None);
            uf.union(id2, id3, None);
            uf.union(id3, id4, None);

            // All should have the same root after path compression
            let root1 = uf.find_root(id1);
            let root2 = uf.find_root(id2);
            let root3 = uf.find_root(id3);
            let root4 = uf.find_root(id4);

            assert_eq!(root1, root2);
            assert_eq!(root2, root3);
            assert_eq!(root3, root4);
        }
    }

    #[test]
    fn test_complex_type_unification() {
        let mut uf = UnionFind::new();

        let var1 = uf.fresh_type_var();
        let var2 = uf.fresh_type_var();

        // Create: Array<T1> and Array<Option<T2>>
        let array1 = Type::Array(Box::new(var1.clone()));
        let array2 = Type::Array(Box::new(Type::Option(Box::new(var2.clone()))));

        uf.unify_types(&array1, &array2).unwrap();

        // T1 should resolve to Option<T2>
        let resolved_var1 = uf.resolve_type(&var1);
        assert!(matches!(resolved_var1, Type::Option(_)));
    }
}
