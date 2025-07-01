use crate::source::Span;
use crate::lexer::TokenKind;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
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
        params: Vec<Param>,
        ret_type: Option<TypeAnn>,
        body: Block,
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
    /// Wildcard pattern `_`
    Wildcard,
    /// Literal pattern `42`, `"hello"`, `true`
    Literal(Literal),
    /// Variable binding pattern `x`
    Identifier(String),
    /// Array destructuring pattern `[a, b, c]`
    Array(Vec<Pattern>),
    /// Object destructuring pattern `{x, y}`
    Object(Vec<ObjectPatternField>),
    /// Or pattern `a | b`
    Or(Vec<Pattern>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectPatternField {
    pub key: String,
    pub pattern: Option<Pattern>, // None means shorthand `{x}` equivalent to `{x: x}`
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Number(f64),
    String(String),
    Boolean(bool),
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Equal,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    And,
    Or,
}

impl BinaryOp {
    pub fn from_token(kind: &TokenKind) -> Option<Self> {
        match kind {
            TokenKind::Plus => Some(BinaryOp::Add),
            TokenKind::Minus => Some(BinaryOp::Subtract),
            TokenKind::Star => Some(BinaryOp::Multiply),
            TokenKind::Slash => Some(BinaryOp::Divide),
            TokenKind::Percent => Some(BinaryOp::Modulo),
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

    pub fn precedence(&self) -> u8 {
        match self {
            BinaryOp::Or => 1,
            BinaryOp::And => 2,
            BinaryOp::Equal | BinaryOp::NotEqual => 3,
            BinaryOp::Less | BinaryOp::Greater | BinaryOp::LessEqual | BinaryOp::GreaterEqual => 4,
            BinaryOp::Add | BinaryOp::Subtract => 5,
            BinaryOp::Multiply | BinaryOp::Divide | BinaryOp::Modulo => 6,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Not,
    Negate,
}

impl UnaryOp {
    pub fn from_token(kind: &TokenKind) -> Option<Self> {
        match kind {
            TokenKind::Bang => Some(UnaryOp::Not),
            TokenKind::Minus => Some(UnaryOp::Negate),
            _ => None,
        }
    }
}

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
    Function {
        params: Vec<TypeAnn>,
        ret: Box<TypeAnn>,
    },
    Array(Box<TypeAnn>),
}

// Display implementations for pretty printing
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
        match &self.kind {
            StmtKind::Let { name, type_ann, init } => {
                write!(f, "let {}", name)?;
                if let Some(ty) = type_ann {
                    write!(f, ": {}", ty)?;
                }
                if let Some(expr) = init {
                    write!(f, " = {}", expr)?;
                }
                Ok(())
            }
            StmtKind::Function { name, params, ret_type, body } => {
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
            StmtKind::For { variable, iterable, body } => {
                write!(f, "for {} in {} {}", variable, iterable, body)
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
            ExprKind::If { condition, then_branch, else_branch } => {
                write!(f, "if {} {{ {} }}", condition, then_branch)?;
                if let Some(else_expr) = else_branch {
                    write!(f, " else {{ {} }}", else_expr)?;
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
                write!(f, "match {} {{", expr)?;
                for arm in arms {
                    write!(f, " {} => {},", arm.pattern, arm.body)?;
                }
                write!(f, " }}")
            }
        }
    }
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        for stmt in &self.statements {
            write!(f, " {}; ", stmt)?;
        }
        if let Some(expr) = &self.final_expr {
            write!(f, " {} ", expr)?;
        }
        write!(f, "}}")
    }
}

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Literal::Number(n) => write!(f, "{}", n),
            Literal::String(s) => write!(f, "\"{}\"", s),
            Literal::Boolean(b) => write!(f, "{}", b),
        }
    }
}

impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BinaryOp::Add => write!(f, "+"),
            BinaryOp::Subtract => write!(f, "-"),
            BinaryOp::Multiply => write!(f, "*"),
            BinaryOp::Divide => write!(f, "/"),
            BinaryOp::Modulo => write!(f, "%"),
            BinaryOp::Equal => write!(f, "=="),
            BinaryOp::NotEqual => write!(f, "!="),
            BinaryOp::Less => write!(f, "<"),
            BinaryOp::Greater => write!(f, ">"),
            BinaryOp::LessEqual => write!(f, "<="),
            BinaryOp::GreaterEqual => write!(f, ">="),
            BinaryOp::And => write!(f, "&&"),
            BinaryOp::Or => write!(f, "||"),
        }
    }
}

impl fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnaryOp::Not => write!(f, "!"),
            UnaryOp::Negate => write!(f, "-"),
        }
    }
}

impl fmt::Display for TypeAnn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            TypeKind::Named(name) => write!(f, "{}", name),
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
            TypeKind::Array(elem_type) => write!(f, "[{}]", elem_type),
        }
    }
}

impl fmt::Display for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            PatternKind::Wildcard => write!(f, "_"),
            PatternKind::Literal(lit) => write!(f, "{}", lit),
            PatternKind::Identifier(name) => write!(f, "{}", name),
            PatternKind::Array(patterns) => {
                write!(f, "[")?;
                for (i, pattern) in patterns.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", pattern)?;
                }
                write!(f, "]")
            }
            PatternKind::Object(fields) => {
                write!(f, "{{")?;
                for (i, field) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", field.key)?;
                    if let Some(pattern) = &field.pattern {
                        write!(f, ": {}", pattern)?;
                    }
                }
                write!(f, "}}")
            }
            PatternKind::Or(patterns) => {
                for (i, pattern) in patterns.iter().enumerate() {
                    if i > 0 {
                        write!(f, " | ")?;
                    }
                    write!(f, "{}", pattern)?;
                }
                Ok(())
            }
        }
    }
}