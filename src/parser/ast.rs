use crate::lexer::TokenKind;
use crate::source::Span;
use std::fmt;

/// Generic type parameter in function or struct definitions
#[derive(Debug, Clone, PartialEq)]
pub struct GenericParam {
    pub name: String,
    pub bounds: Vec<TraitBound>,
    pub span: Span,
}

/// Trait bound on a generic parameter
#[derive(Debug, Clone, PartialEq)]
pub struct TraitBound {
    pub trait_name: String,
    pub span: Span,
}

/// Collection of generic parameters
#[derive(Debug, Clone, PartialEq)]
pub struct GenericParams {
    pub params: Vec<GenericParam>,
    pub span: Span,
}

/// Field in a struct declaration
#[derive(Debug, Clone, PartialEq)]
pub struct StructField {
    pub name: String,
    pub type_ann: TypeAnn,
    pub span: Span,
}

/// Variant in an enum declaration
#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariant {
    pub name: String,
    pub fields: EnumVariantFields,
    pub span: Span,
}

/// Different kinds of enum variant fields
#[derive(Debug, Clone, PartialEq)]
pub enum EnumVariantFields {
    /// Unit variant: `None`
    Unit,
    /// Tuple variant: `Some(T)`
    Tuple(Vec<TypeAnn>),
    /// Struct variant: `Point { x: i32, y: i32 }`
    Struct(Vec<StructField>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Attribute {
    pub name: String,
    pub args: Vec<String>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
    pub attributes: Vec<Attribute>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    Let {
        name: String,
        type_ann: Option<TypeAnn>,
        init: Option<Expr>,
    },
    Function {
        name: String,
        generic_params: Option<GenericParams>,
        params: Vec<Param>,
        ret_type: Option<TypeAnn>,
        body: Block,
        is_async: bool,
    },
    Return(Option<Expr>),
    Expression(Expr),
    While {
        condition: Expr,
        body: Block,
    },
    For {
        variable: String,
        iterable: Expr,
        body: Block,
    },
    Import {
        imports: ImportSpec,
        module: String,
    },
    Export {
        export: ExportSpec,
    },
    Struct {
        name: String,
        generic_params: Option<GenericParams>,
        fields: Vec<StructField>,
    },
    Enum {
        name: String,
        generic_params: Option<GenericParams>,
        variants: Vec<EnumVariant>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    Literal(Literal),
    Identifier(String),
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
    },
    Member {
        object: Box<Expr>,
        property: String,
    },
    If {
        condition: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Option<Box<Expr>>,
    },
    Block(Block),
    Array(Vec<Expr>),
    Assign {
        target: Box<Expr>,
        value: Box<Expr>,
    },
    Match {
        expr: Box<Expr>,
        arms: Vec<MatchArm>,
    },
    Await {
        expr: Box<Expr>,
    },
    ListComprehension {
        element: Box<Expr>,
        variable: String,
        iterable: Box<Expr>,
        condition: Option<Box<Expr>>,
    },
    /// Generic constructor expression (e.g., Vec<i32>, HashMap<String, T>)
    GenericConstructor {
        name: String,
        type_args: Vec<TypeAnn>,
    },
    /// Struct constructor expression (e.g., Point { x: 1, y: 2 })
    StructConstructor {
        name: String,
        fields: Vec<(String, Expr)>,
    },
    /// Enum variant constructor (e.g., Some(5), None)
    EnumConstructor {
        enum_name: Option<String>,  // None for unqualified variants
        variant: String,
        args: EnumConstructorArgs,
    },
}

/// Arguments for enum variant constructor
#[derive(Debug, Clone, PartialEq)]
pub enum EnumConstructorArgs {
    /// Unit variant: None
    Unit,
    /// Tuple variant: Some(5)
    Tuple(Vec<Expr>),
    /// Struct variant: Point { x: 1, y: 2 }
    Struct(Vec<(String, Expr)>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub statements: Vec<Stmt>,
    pub final_expr: Option<Box<Expr>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Expr>,
    pub body: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Pattern {
    pub kind: PatternKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PatternKind {
    Wildcard,
    Identifier(String),
    Literal(Literal),
    Array(Vec<Pattern>),
    Object(Vec<(String, Option<Pattern>)>),
    Or(Vec<Pattern>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ImportSpecifier {
    /// Default import: import name from "module"
    Default { name: String },
    /// Named import: import { name as alias } from "module"
    Named { name: String, alias: Option<String> },
    /// Namespace import: import * as name from "module"
    Namespace { alias: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExportSpecifier {
    pub name: String,
    pub alias: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExportKind {
    /// Named exports: export { a, b as c }
    Named { specifiers: Vec<ExportSpecifier> },
    /// Function export: export fn foo() {}
    Function {
        name: String,
        params: Vec<Param>,
        ret_type: Option<TypeAnn>,
        body: Block,
        is_async: bool,
    },
    /// Variable export: export let x = 1
    Variable {
        name: String,
        type_ann: Option<TypeAnn>,
        init: Option<Expr>,
    },
    /// Default export: export default expr
    Default { expr: Expr },
    /// Declaration export: export let x = 1 or export fn foo() {}
    Declaration(Box<Stmt>),
}

// Type aliases for compatibility
pub type ImportSpec = Vec<ImportSpecifier>;
pub type ExportSpec = ExportKind;
pub type ExportItem = ExportSpecifier;

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub type_ann: TypeAnn,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeAnn {
    pub kind: TypeKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeKind {
    Named(String),
    Array(Box<TypeAnn>),
    Function {
        params: Vec<TypeAnn>,
        ret: Box<TypeAnn>,
    },
    /// Generic type with type arguments (e.g., Vec<i32>, Map<string, i32>)
    Generic {
        name: String,
        args: Vec<TypeAnn>,
    },
    /// Type parameter in generic context (e.g., T, U, K, V)
    TypeParam(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Number(f64),
    String(String),
    Boolean(bool),
    Null,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinaryOp {
    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Mod,

    // Comparison
    Equal,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,

    // Logical
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryOp {
    Not,
    Minus,
}

// Display implementations
impl fmt::Display for Attribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "@{}", self.name)?;
        if !self.args.is_empty() {
            write!(f, "(")?;
            for (i, arg) in self.args.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", arg)?;
            }
            write!(f, ")")?;
        }
        Ok(())
    }
}

impl fmt::Display for Program {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, stmt) in self.statements.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "{}", stmt)?;
        }
        Ok(())
    }
}

impl fmt::Display for Stmt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Display attributes
        for attr in &self.attributes {
            writeln!(f, "{}", attr)?;
        }

        // Display statement
        match &self.kind {
            StmtKind::Let {
                name,
                type_ann,
                init,
            } => {
                write!(f, "let {}", name)?;
                if let Some(ty) = type_ann {
                    write!(f, ": {}", ty)?;
                }
                if let Some(expr) = init {
                    write!(f, " = {}", expr)?;
                }
                Ok(())
            }
            StmtKind::Function {
                name,
                params,
                ret_type,
                body,
                is_async,
                generic_params,
            } => {
                if *is_async {
                    write!(f, "async ")?;
                }
                write!(f, "fn {}", name)?;
                
                // Display generic parameters
                if let Some(generics) = generic_params {
                    write!(f, "{}", generics)?;
                }
                
                write!(f, "(")?;
                for (i, param) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", param.name, param.type_ann)?;
                }
                write!(f, ")")?;
                if let Some(ret) = ret_type {
                    write!(f, " -> {}", ret)?;
                }
                write!(f, " {}", body)
            }
            StmtKind::Return(expr) => {
                write!(f, "return")?;
                if let Some(e) = expr {
                    write!(f, " {}", e)?;
                }
                Ok(())
            }
            StmtKind::Expression(expr) => write!(f, "{}", expr),
            StmtKind::While { condition, body } => {
                write!(f, "while {} {}", condition, body)
            }
            StmtKind::For {
                variable,
                iterable,
                body,
            } => {
                write!(f, "for {} in {} {}", variable, iterable, body)
            }
            StmtKind::Import { imports, module } => {
                write!(f, "import ")?;

                // Handle mixed imports (default + named/namespace)
                let mut has_default = false;
                let mut named_specs = Vec::new();
                let mut namespace_spec = None;

                for spec in imports {
                    match spec {
                        ImportSpecifier::Default { name } => {
                            write!(f, "{}", name)?;
                            has_default = true;
                        }
                        ImportSpecifier::Named { name, alias } => {
                            named_specs.push((name, alias));
                        }
                        ImportSpecifier::Namespace { alias } => {
                            namespace_spec = Some(alias);
                        }
                    }
                }

                // Add comma after default if there are more imports
                if has_default && (!named_specs.is_empty() || namespace_spec.is_some()) {
                    write!(f, ", ")?;
                }

                // Handle named imports
                if !named_specs.is_empty() {
                    write!(f, "{{ ")?;
                    for (i, (name, alias)) in named_specs.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", name)?;
                        if let Some(a) = alias {
                            write!(f, " as {}", a)?;
                        }
                    }
                    write!(f, " }}")?;

                    // Add comma if there's also a namespace import
                    if namespace_spec.is_some() {
                        write!(f, ", ")?;
                    }
                }

                // Handle namespace import
                if let Some(alias) = namespace_spec {
                    write!(f, "* as {}", alias)?;
                }

                write!(f, " from \"{}\"", module)
            }
            StmtKind::Export { export } => {
                write!(f, "export ")?;
                match export {
                    ExportKind::Named { specifiers } => {
                        write!(f, "{{ ")?;
                        for (i, spec) in specifiers.iter().enumerate() {
                            if i > 0 {
                                write!(f, ", ")?;
                            }
                            write!(f, "{}", spec.name)?;
                            if let Some(alias) = &spec.alias {
                                write!(f, " as {}", alias)?;
                            }
                        }
                        write!(f, " }}")
                    }
                    ExportKind::Function {
                        name,
                        params,
                        ret_type,
                        body,
                        is_async,
                    } => {
                        if *is_async {
                            write!(f, "async ")?;
                        }
                        write!(f, "fn {}(", name)?;
                        for (i, param) in params.iter().enumerate() {
                            if i > 0 {
                                write!(f, ", ")?;
                            }
                            write!(f, "{}: {}", param.name, param.type_ann)?;
                        }
                        write!(f, ")")?;
                        if let Some(ret) = ret_type {
                            write!(f, " -> {}", ret)?;
                        }
                        write!(f, " {}", body)
                    }
                    ExportKind::Variable {
                        name,
                        type_ann,
                        init,
                    } => {
                        write!(f, "let {}", name)?;
                        if let Some(ty) = type_ann {
                            write!(f, ": {}", ty)?;
                        }
                        if let Some(expr) = init {
                            write!(f, " = {}", expr)?;
                        }
                        Ok(())
                    }
                    ExportKind::Default { expr } => write!(f, "default {}", expr),
                    ExportKind::Declaration(stmt) => write!(f, "{}", stmt),
                }
            }
            StmtKind::Struct { name, generic_params, fields } => {
                write!(f, "struct {}", name)?;
                if let Some(generics) = generic_params {
                    write!(f, "{}", generics)?;
                }
                write!(f, " {{")?;
                for (i, field) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, " {}: {}", field.name, field.type_ann)?;
                }
                write!(f, " }}")
            }
            StmtKind::Enum { name, generic_params, variants } => {
                write!(f, "enum {}", name)?;
                if let Some(generics) = generic_params {
                    write!(f, "{}", generics)?;
                }
                write!(f, " {{")?;
                for (i, variant) in variants.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, " {}", variant)?;
                }
                write!(f, " }}")
            }
        }
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            ExprKind::Literal(lit) => write!(f, "{}", lit),
            ExprKind::Identifier(name) => write!(f, "{}", name),
            ExprKind::Binary { left, op, right } => {
                write!(f, "({} {} {})", left, op, right)
            }
            ExprKind::Unary { op, expr } => {
                write!(f, "({}{})", op, expr)
            }
            ExprKind::Call { callee, args } => {
                write!(f, "{}(", callee)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
            ExprKind::Index { object, index } => {
                write!(f, "{}[{}]", object, index)
            }
            ExprKind::Member { object, property } => {
                write!(f, "{}.{}", object, property)
            }
            ExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                write!(f, "if {} {{ {} }}", condition, then_branch)?;
                if let Some(else_br) = else_branch {
                    write!(f, " else {{ {} }}", else_br)?;
                }
                Ok(())
            }
            ExprKind::Block(block) => write!(f, "{}", block),
            ExprKind::Array(elements) => {
                write!(f, "[")?;
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", elem)?;
                }
                write!(f, "]")
            }
            ExprKind::Assign { target, value } => {
                write!(f, "{} = {}", target, value)
            }
            ExprKind::Match { expr, arms } => {
                write!(f, "match {} {{ ", expr)?;
                for arm in arms {
                    write!(f, "{} => {}, ", arm.pattern, arm.body)?;
                }
                write!(f, "}}")
            }
            ExprKind::Await { expr } => {
                write!(f, "await {}", expr)
            }
            ExprKind::ListComprehension {
                element,
                variable,
                iterable,
                condition,
            } => {
                write!(f, "[{} for {} in {}", element, variable, iterable)?;
                if let Some(cond) = condition {
                    write!(f, " if {}", cond)?;
                }
                write!(f, "]")
            }
            ExprKind::GenericConstructor { name, type_args } => {
                write!(f, "{}", name)?;
                if !type_args.is_empty() {
                    write!(f, "<")?;
                    for (i, arg) in type_args.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", arg)?;
                    }
                    write!(f, ">")?;
                }
                Ok(())
            }
            ExprKind::StructConstructor { name, fields } => {
                write!(f, "{} {{ ", name)?;
                for (i, (field_name, expr)) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", field_name, expr)?;
                }
                write!(f, " }}")
            }
            ExprKind::EnumConstructor { enum_name, variant, args } => {
                if let Some(enum_name) = enum_name {
                    write!(f, "{}::{}", enum_name, variant)?;
                } else {
                    write!(f, "{}", variant)?;
                }
                match args {
                    EnumConstructorArgs::Unit => Ok(()),
                    EnumConstructorArgs::Tuple(exprs) => {
                        write!(f, "(")?;
                        for (i, expr) in exprs.iter().enumerate() {
                            if i > 0 {
                                write!(f, ", ")?;
                            }
                            write!(f, "{}", expr)?;
                        }
                        write!(f, ")")
                    }
                    EnumConstructorArgs::Struct(fields) => {
                        write!(f, " {{ ")?;
                        for (i, (field_name, expr)) in fields.iter().enumerate() {
                            if i > 0 {
                                write!(f, ", ")?;
                            }
                            write!(f, "{}: {}", field_name, expr)?;
                        }
                        write!(f, " }}")
                    }
                }
            }
        }
    }
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{{")?;
        for stmt in &self.statements {
            writeln!(f, "    {}", stmt)?;
        }
        if let Some(expr) = &self.final_expr {
            writeln!(f, "    {}", expr)?;
        }
        write!(f, "}}")
    }
}

impl fmt::Display for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            PatternKind::Wildcard => write!(f, "_"),
            PatternKind::Identifier(name) => write!(f, "{}", name),
            PatternKind::Literal(lit) => write!(f, "{}", lit),
            PatternKind::Array(patterns) => {
                write!(f, "[")?;
                for (i, p) in patterns.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", p)?;
                }
                write!(f, "]")
            }
            PatternKind::Object(fields) => {
                write!(f, "{{")?;
                for (i, (name, pat)) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", name)?;
                    if let Some(p) = pat {
                        write!(f, ": {}", p)?;
                    }
                }
                write!(f, "}}")
            }
            PatternKind::Or(patterns) => {
                for (i, p) in patterns.iter().enumerate() {
                    if i > 0 {
                        write!(f, " | ")?;
                    }
                    write!(f, "{}", p)?;
                }
                Ok(())
            }
        }
    }
}

impl fmt::Display for TypeAnn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            TypeKind::Named(name) => write!(f, "{}", name),
            TypeKind::Array(elem) => write!(f, "[{}]", elem),
            TypeKind::Function { params, ret } => {
                write!(f, "(")?;
                for (i, param) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", param)?;
                }
                write!(f, ") -> {}", ret)
            }
            TypeKind::Generic { name, args } => {
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
            TypeKind::TypeParam(name) => write!(f, "{}", name),
        }
    }
}

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Literal::Number(n) => write!(f, "{}", n),
            Literal::String(s) => write!(f, "\"{}\"", s),
            Literal::Boolean(b) => write!(f, "{}", b),
            Literal::Null => write!(f, "null"),
        }
    }
}

impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let op = match self {
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::Mod => "%",
            BinaryOp::Equal => "==",
            BinaryOp::NotEqual => "!=",
            BinaryOp::Less => "<",
            BinaryOp::Greater => ">",
            BinaryOp::LessEqual => "<=",
            BinaryOp::GreaterEqual => ">=",
            BinaryOp::And => "&&",
            BinaryOp::Or => "||",
        };
        write!(f, "{}", op)
    }
}

impl fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let op = match self {
            UnaryOp::Not => "!",
            UnaryOp::Minus => "-",
        };
        write!(f, "{}", op)
    }
}

impl BinaryOp {
    pub fn from_token(token: &TokenKind) -> Option<Self> {
        match token {
            TokenKind::Plus => Some(BinaryOp::Add),
            TokenKind::Minus => Some(BinaryOp::Sub),
            TokenKind::Star => Some(BinaryOp::Mul),
            TokenKind::Slash => Some(BinaryOp::Div),
            TokenKind::Percent => Some(BinaryOp::Mod),
            TokenKind::EqualsEquals => Some(BinaryOp::Equal),
            TokenKind::BangEquals => Some(BinaryOp::NotEqual),
            TokenKind::Less => Some(BinaryOp::Less),
            TokenKind::Greater => Some(BinaryOp::Greater),
            TokenKind::LessEquals => Some(BinaryOp::LessEqual),
            TokenKind::GreaterEquals => Some(BinaryOp::GreaterEqual),
            TokenKind::And => Some(BinaryOp::And),
            TokenKind::Or => Some(BinaryOp::Or),
            _ => None,
        }
    }
}

impl UnaryOp {
    pub fn from_token(token: &TokenKind) -> Option<Self> {
        match token {
            TokenKind::Bang => Some(UnaryOp::Not),
            TokenKind::Minus => Some(UnaryOp::Minus),
            _ => None,
        }
    }
}

impl fmt::Display for ImportSpecifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImportSpecifier::Named { name, alias } => {
                write!(f, "{}", name)?;
                if let Some(alias) = alias {
                    write!(f, " as {}", alias)?;
                }
                Ok(())
            }
            ImportSpecifier::Namespace { alias } => write!(f, "* as {}", alias),
            ImportSpecifier::Default { name } => write!(f, "{}", name),
        }
    }
}

impl fmt::Display for ExportKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExportKind::Named { specifiers } => {
                write!(f, "{{ ")?;
                for (i, item) in specifiers.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", item.name)?;
                    if let Some(alias) = &item.alias {
                        write!(f, " as {}", alias)?;
                    }
                }
                write!(f, " }}")
            }
            ExportKind::Function {
                name,
                params,
                ret_type,
                body,
                is_async,
            } => {
                if *is_async {
                    write!(f, "async ")?;
                }
                write!(f, "fn {}(", name)?;
                for (i, param) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", param.name, param.type_ann)?;
                }
                write!(f, ")")?;
                if let Some(ret) = ret_type {
                    write!(f, " -> {}", ret)?;
                }
                write!(f, " {}", body)
            }
            ExportKind::Variable {
                name,
                type_ann,
                init,
            } => {
                write!(f, "let {}", name)?;
                if let Some(ty) = type_ann {
                    write!(f, ": {}", ty)?;
                }
                if let Some(expr) = init {
                    write!(f, " = {}", expr)?;
                }
                Ok(())
            }
            ExportKind::Default { expr } => write!(f, "default {}", expr),
            ExportKind::Declaration(stmt) => write!(f, "{}", stmt),
        }
    }
}

impl fmt::Display for GenericParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<")?;
        for (i, param) in self.params.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", param)?;
        }
        write!(f, ">")
    }
}

impl fmt::Display for GenericParam {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)?;
        if !self.bounds.is_empty() {
            write!(f, ": ")?;
            for (i, bound) in self.bounds.iter().enumerate() {
                if i > 0 {
                    write!(f, " + ")?;
                }
                write!(f, "{}", bound)?;
            }
        }
        Ok(())
    }
}

impl fmt::Display for TraitBound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.trait_name)
    }
}

impl fmt::Display for StructField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.name, self.type_ann)
    }
}

impl fmt::Display for EnumVariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)?;
        match &self.fields {
            EnumVariantFields::Unit => Ok(()),
            EnumVariantFields::Tuple(types) => {
                write!(f, "(")?;
                for (i, ty) in types.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", ty)?;
                }
                write!(f, ")")
            }
            EnumVariantFields::Struct(fields) => {
                write!(f, " {{")?;
                for (i, field) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, " {}", field)?;
                }
                write!(f, " }}")
            }
        }
    }
}

impl fmt::Display for EnumVariantFields {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EnumVariantFields::Unit => Ok(()),
            EnumVariantFields::Tuple(types) => {
                write!(f, "(")?;
                for (i, ty) in types.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", ty)?;
                }
                write!(f, ")")
            }
            EnumVariantFields::Struct(fields) => {
                write!(f, "{{")?;
                for (i, field) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, " {}: {}", field.name, field.type_ann)?;
                }
                write!(f, " }}")
            }
        }
    }
}
