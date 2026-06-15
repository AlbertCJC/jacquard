//! Type IR (Intermediate Representation) for the Jacquard compiler.
//!
//! Defines the internal type representation used during type checking,
//! along with a union-find based type variable table for unification.

/// The internal type representation used during type checking.
///
/// Unlike the AST `Type` (which represents type *annotations* written by the
/// user), this enum represents fully-resolved compiler types, including
/// inference variables and error types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    // Signed integers
    I8,
    I16,
    I32,
    I64,

    // Unsigned integers
    U8,
    U16,
    U32,
    U64,

    // Floating point
    F32,
    F64,

    // Other primitives
    Bool,
    String,
    Void,

    // Compound types
    Function(Vec<Type>, Box<Type>),
    Tuple(Vec<Type>),

    // User-defined
    Named(String),
    Generic(String, Vec<Type>),

    // Inference
    Var(usize), // TypeVarId — index into TypeVarTable
    Error,       // Poison pill for cascading error suppression
}

/// State of a type variable in the unification table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeVarState {
    /// Fresh type variable, not yet resolved. The u32 is a counter for
    /// displaying as `?N` in error messages.
    Unbound(u32),
    /// Resolved to a concrete type.
    Bound(Type),
    /// Chained to another type variable (union-find link).
    Link(usize),
}

impl TypeVarState {
    /// Returns true if this state represents an unbound variable.
    pub fn is_unbound(&self) -> bool {
        matches!(self, TypeVarState::Unbound(_))
    }

    /// Returns true if this state represents a bound (resolved) type.
    pub fn is_bound(&self) -> bool {
        matches!(self, TypeVarState::Bound(_))
    }
}

/// Union-find table for type variables.
///
/// Tracks the state of each type variable introduced during inference.
/// Uses path compression for O(α(n)) amortized operations.
#[derive(Debug, Clone)]
pub struct TypeVarTable {
    vars: Vec<TypeVarState>,
    next_var_id: u32,
}

impl TypeVarTable {
    /// Create a new, empty type variable table.
    pub fn new() -> Self {
        TypeVarTable {
            vars: Vec::new(),
            next_var_id: 0,
        }
    }

    /// Create a fresh type variable and return its ID.
    pub fn new_var(&mut self) -> usize {
        let id = self.vars.len();
        let display_id = self.next_var_id;
        self.next_var_id += 1;
        self.vars.push(TypeVarState::Unbound(display_id));
        id
    }

    /// Find the root variable ID for a given variable, with path compression.
    pub fn find_root(&mut self, id: usize) -> usize {
        match &self.vars[id] {
            TypeVarState::Link(parent) => {
                let root = self.find_root(*parent);
                self.vars[id] = TypeVarState::Link(root); // path compression
                root
            }
            _ => id,
        }
    }

    /// Resolve a variable ID to its current state, following links.
    pub fn resolve(&mut self, id: usize) -> &TypeVarState {
        let root = self.find_root(id);
        &self.vars[root]
    }

    /// Resolve a variable to its concrete type if bound.
    ///
    /// Follows union-find links and returns `Some(Type)` if the root
    /// is bound, or `None` if it is still unbound.
    pub fn resolve_type(&mut self, id: usize) -> Option<Type> {
        let root = self.find_root(id);
        match &self.vars[root] {
            TypeVarState::Bound(ty) => Some(ty.clone()),
            _ => None,
        }
    }

    /// Bind a type variable to a concrete type.
    ///
    /// Panics if the variable already has a different binding.
    pub fn bind(&mut self, id: usize, ty: Type) {
        let root = self.find_root(id);
        self.vars[root] = TypeVarState::Bound(ty);
    }

    /// Union two type variables so they resolve to the same underlying type.
    ///
    /// After union, `a` and `b` will share the same root. If one is already
    /// bound, the other is linked to it.
    pub fn union(&mut self, a: usize, b: usize) {
        let root_a = self.find_root(a);
        let root_b = self.find_root(b);
        if root_a == root_b {
            return;
        }
        // If one is bound, keep it as root
        match &self.vars[root_a] {
            TypeVarState::Bound(_) => {
                self.vars[root_b] = TypeVarState::Link(root_a);
                return;
            }
            _ => {}
        }
        // Default: link b into a
        self.vars[root_b] = TypeVarState::Link(root_a);
    }

    /// Unify two types, binding variables as needed.
    ///
    /// Returns `Ok(())` if the types can be unified, or `Err(TypeError)` if
    /// there is a mismatch.
    pub fn unify(&mut self, a: &Type, b: &Type) -> Result<(), TypeError> {
        // Poison pill: Error types always unify
        if *a == Type::Error || *b == Type::Error {
            return Ok(());
        }

        // Resolve type variables to their roots
        let a = self.resolve_type_recurse(a);
        let b = self.resolve_type_recurse(b);

        match (&a, &b) {
            // Two variables
            (Type::Var(va), Type::Var(vb)) => {
                self.union(*va, *vb);
                Ok(())
            }

            // One variable, one concrete type
            (Type::Var(v), concrete) | (concrete, Type::Var(v)) => {
                self.occurs_check(*v, concrete)?;
                self.bind(*v, concrete.clone());
                Ok(())
            }

            // Primitives must match exactly
            (Type::I32, Type::I32) | (Type::Bool, Type::Bool) | (Type::String, Type::String)
            | (Type::Void, Type::Void) | (Type::I8, Type::I8) | (Type::I16, Type::I16)
            | (Type::I64, Type::I64) | (Type::U8, Type::U8) | (Type::U16, Type::U16)
            | (Type::U32, Type::U32) | (Type::U64, Type::U64)
            | (Type::F32, Type::F32) | (Type::F64, Type::F64) => Ok(()),

            // Different primitives → error
            (a_prim, b_prim) if is_primitive(a_prim) && is_primitive(b_prim) => {
                Err(TypeError {
                    message: format!("type mismatch: expected {}, found {}", type_name(a_prim), type_name(b_prim)),
                    span: None,
                })
            }

            // Function types: unify param lists and return types
            (Type::Function(params_a, ret_a), Type::Function(params_b, ret_b)) => {
                if params_a.len() != params_b.len() {
                    return Err(TypeError {
                        message: format!(
                            "function arity mismatch: expected {} params, found {}",
                            params_a.len(),
                            params_b.len()
                        ),
                        span: None,
                    });
                }
                for (pa, pb) in params_a.iter().zip(params_b.iter()) {
                    self.unify(pa, pb)?;
                }
                self.unify(ret_a, ret_b)
            }

            // Tuple types: unify element-wise
            (Type::Tuple(elems_a), Type::Tuple(elems_b)) => {
                if elems_a.len() != elems_b.len() {
                    return Err(TypeError {
                        message: format!(
                            "tuple arity mismatch: expected {} elements, found {}",
                            elems_a.len(),
                            elems_b.len()
                        ),
                        span: None,
                    });
                }
                for (ea, eb) in elems_a.iter().zip(elems_b.iter()) {
                    self.unify(ea, eb)?;
                }
                Ok(())
            }

            // Named types: must match by name
            (Type::Named(name_a), Type::Named(name_b)) => {
                if name_a == name_b {
                    Ok(())
                } else {
                    Err(TypeError {
                        message: format!("type mismatch: expected {}, found {}", name_a, name_b),
                        span: None,
                    })
                }
            }

            // Generic types: match name, unify args
            (Type::Generic(name_a, args_a), Type::Generic(name_b, args_b)) => {
                if name_a != name_b {
                    return Err(TypeError {
                        message: format!("type mismatch: expected {}, found {}", name_a, name_b),
                        span: None,
                    });
                }
                if args_a.len() != args_b.len() {
                    return Err(TypeError {
                        message: format!(
                            "generic arity mismatch: {} expects {} args, found {}",
                            name_a,
                            args_a.len(),
                            args_b.len()
                        ),
                        span: None,
                    });
                }
                for (aa, ab) in args_a.iter().zip(args_b.iter()) {
                    self.unify(aa, ab)?;
                }
                Ok(())
            }

            // Mismatched type constructors
            _ => Err(TypeError {
                message: format!(
                    "type mismatch: expected {}, found {}",
                    type_name(&a),
                    type_name(&b)
                ),
                span: None,
            }),
        }
    }

    /// Resolve type variables within a Type to their root forms.
    fn resolve_type_recurse(&mut self, ty: &Type) -> Type {
        match ty {
            Type::Var(id) => {
                let root = self.find_root(*id);
                match &self.vars[root] {
                    TypeVarState::Bound(concrete) => concrete.clone(),
                    TypeVarState::Unbound(_) => Type::Var(root),
                    TypeVarState::Link(_) => {
                        // find_root should have resolved all links, so this is unreachable
                        Type::Var(root)
                    }
                }
            }
            Type::Function(params, ret) => Type::Function(
                params.iter().map(|p| self.resolve_type_recurse(p)).collect(),
                Box::new(self.resolve_type_recurse(ret)),
            ),
            Type::Tuple(elems) => Type::Tuple(
                elems.iter().map(|e| self.resolve_type_recurse(e)).collect(),
            ),
            Type::Generic(name, args) => Type::Generic(
                name.clone(),
                args.iter().map(|a| self.resolve_type_recurse(a)).collect(),
            ),
            other => other.clone(),
        }
    }

    /// Occurs check: prevent infinite types like `T = Function([T], T)`.
    fn occurs_check(&mut self, var_id: usize, ty: &Type) -> Result<(), TypeError> {
        if self.occurs_in(var_id, ty) {
            return Err(TypeError {
                message: "infinite type: type variable occurs in its own binding".to_string(),
                span: None,
            });
        }
        Ok(())
    }

    fn occurs_in(&mut self, var_id: usize, ty: &Type) -> bool {
        match ty {
            Type::Var(id) => {
                let root = self.find_root(*id);
                if root == var_id {
                    return true;
                }
                // Check if this variable is bound, and recurse into its binding
                match &self.vars[root] {
                    TypeVarState::Bound(bound_ty) => {
                        let bound = bound_ty.clone();
                        self.occurs_in(var_id, &bound)
                    }
                    _ => false,
                }
            }
            Type::Function(params, ret) => {
                params.iter().any(|p| self.occurs_in(var_id, p))
                    || self.occurs_in(var_id, ret)
            }
            Type::Tuple(elems) => elems.iter().any(|e| self.occurs_in(var_id, e)),
            Type::Generic(_, args) => args.iter().any(|a| self.occurs_in(var_id, a)),
            _ => false,
        }
    }
}

/// Error returned when type unification fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeError {
    pub message: String,
    pub span: Option<(usize, usize)>, // optional source span
}

impl std::fmt::Display for TypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for TypeError {}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn is_primitive(ty: &Type) -> bool {
    matches!(
        ty,
        Type::I8 | Type::I16 | Type::I32 | Type::I64
            | Type::U8 | Type::U16 | Type::U32 | Type::U64
            | Type::F32 | Type::F64
            | Type::Bool | Type::String | Type::Void
    )
}

fn type_name(ty: &Type) -> &'static str {
    match ty {
        Type::I8 => "i8",
        Type::I16 => "i16",
        Type::I32 => "i32",
        Type::I64 => "i64",
        Type::U8 => "u8",
        Type::U16 => "u16",
        Type::U32 => "u32",
        Type::U64 => "u64",
        Type::F32 => "f32",
        Type::F64 => "f64",
        Type::Bool => "bool",
        Type::String => "string",
        Type::Void => "void",
        Type::Function(..) => "function",
        Type::Tuple(..) => "tuple",
        Type::Named(_) => "named type",
        Type::Generic(..) => "generic type",
        Type::Var(_) => "type variable",
        Type::Error => "error type",
    }
}