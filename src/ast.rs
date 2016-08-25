#[allow(dead_code)]
use std::fmt::{Display, Formatter, Error};

// https://docs.python.org/3.5/library/ast.html

pub type Identifier = String;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Stmt {  // boxed as we don't know the size of the trees at compile time
    // simplification from "args: Box<Arguments>"
    FunctionDef{name: Identifier, args: Vec<String>, body: Vec<Stmt>, decorator_list: Vec<Expr>,
                returns: Option<Expr>, location: String},
    Return{value: Option<Expr>, location: String},
    // simplification from "targets: Vec<Expr>", so no unpacking assingments possible a, b = 1, 2
    Assign{target: Box<Expr>, value: Box<Expr>, location: String},
    AugAssign{target: Box<Expr>, op: Operator, value: Box<Expr>, location: String},
    // for-loops are not supported as they need iterators For(target: Box<Expr>, iter: Box<Expr>, body: Vec<Stmt>, orelse: Vec<Stmt>, location: String),
    While{test: Box<Expr>, body: Vec<Stmt>, orelse: Vec<Stmt>, location: String},
    If{test: Box<Expr>, body: Vec<Stmt>, orelse: Vec<Stmt>, location: String},
    // with-blocks are not needed as e.g. opening files is impossible: With(withitem* items, stmt* body)
    // instead of raise there is a custom error()-function: Raise(expr? exc, expr? cause)
    // try-catch is impossible: Try(stmt* body, excepthandler* handlers, stmt* orelse, stmt* finalbody)
    // asserts would need to raise exceptions: Assert(expr test, expr? msg)
    Global{names: Vec<Identifier>, location: String},
    Nonlocal{names: Vec<Identifier>, location: String}, // also not supported but parsed for a better error message about scopes
    Expr{value: Box<Expr>, location: String},
    Pass{location: String}, Break{location: String}, Continue{location: String},
}

impl Stmt {
    pub fn location(&self) -> String {
        match *self {
            Stmt::FunctionDef{name: _, args: _, body: _, decorator_list: _, returns: _, ref location} => location.clone(),
            Stmt::Return{value: _, ref location} => location.clone(),
            Stmt::Assign{target: _, value: _, ref location} => location.clone(),
            Stmt::AugAssign{target: _, op: _, value: _, ref location} => location.clone(),
            Stmt::While{test: _, body: _, orelse: _, ref location} => location.clone(),
            Stmt::If{test: _, body: _, orelse: _, ref location} => location.clone(),
            Stmt::Global{names: _, ref location} => location.clone(),
            Stmt::Nonlocal{names: _, ref location} => location.clone(),
            Stmt::Expr{value: _, ref location} => location.clone(),
            Stmt::Pass{ref location} => location.clone(),
            Stmt::Break{ref location} => location.clone(),
            Stmt::Continue{ref location} => location.clone(),
        }
    }
}


#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Expr {
    BoolOpE{op: BoolOp, values: Vec<Expr>, location: String},
    BinOp{left: Box<Expr>, op: Operator, right: Box<Expr>, location: String},
    UnaryOpE{op: UnaryOp, operand: Box<Expr>, location: String},
    // lambda is not possible because of non-local scopes: Lambda(arguments args, expr body)
    // @TODO: IfExp(test: Box<Expr>, body: Box<Expr>, orelse: Box<Expr>),
    Dict{keys: Vec<Expr>, values: Vec<Expr>, location: String},  // not supported but needed for parsing in comp section
    // simplification: Set(expr* elts)
    // simplification, lists are not supported except for H and M,
    //   but there are helper functions for arrays on M and H: ListComp(expr elt, comprehension* generators)
    Compare{left: Box<Expr>, ops: Vec<CmpOp>, comparators: Vec<Expr>, location: String},
    Call{func: Identifier, args: Vec<Expr>, keywords: Vec<Keyword>, location: String}, // simplification from func: Box<Expr>
    Str{s: String, location: String},
    Num{n: u32, location: String},  // or var?
    NameConstant{value: u32, location: String},
    Ellipsis{location: String},
    Attribute{value: Box<Expr>, attr: Identifier, ctx: ExprContext, location: String},
    Subscript{value: Box<Expr>, slice: Box<Slice>, ctx: ExprContext, location: String},
    Starred{value: Box<Expr>, ctx: ExprContext, location: String},
    Name{id: Identifier, ctx: ExprContext, location: String},
    List{elts: Vec<Expr>, ctx: ExprContext, location: String},
    Tuple{elts: Vec<Expr>, ctx: ExprContext, location: String},
}



impl Expr {
    pub fn location(&self) -> String {
        match *self {
            Expr::BoolOpE{op: _, values: _,  ref location} => location.clone(),
            Expr::BinOp{left: _, op: _, right: _, ref location} => location.clone(),
            Expr::UnaryOpE{op: _, operand:_, ref location} => location.clone(),
            Expr::Dict{keys: _, values: _,  ref location} => location.clone(),
            Expr::Compare{left: _, ops:_, comparators: _, ref location} => location.clone(),
            Expr::Call{func: _, args: _, keywords: _, ref location} => location.clone(),
            Expr::Str{s: _,  ref location} => location.clone(),
            Expr::Num{n: _, ref location} => location.clone(),
            Expr::NameConstant{value: _, ref location} => location.clone(),
            Expr::Ellipsis{ref location} => location.clone(),
            Expr::Attribute{value: _, attr: _, ctx: _, ref location} => location.clone(),
            Expr::Subscript{value: _, slice: _, ctx: _, ref location} => location.clone(),
            Expr::Starred{value: _, ctx: _, ref location} => location.clone(),
            Expr::Name{id: _, ctx: _, ref location} => location.clone(),
            Expr::List{elts: _, ctx: _, ref location} => location.clone(),
            Expr::Tuple{elts: _, ctx: _, ref location} => location.clone(),
        }
    }
}

#[allow(dead_code)]
#[derive(Copy, Clone, Debug)]
pub enum ExprContext { Load, Store, Del, AugLoad, AugStore, Param } // only Load and Store are used

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Slice {
    Slice{lower: Option<Expr>, upper: Option<Expr>, step: Option<Expr>}, // not supported
    ExtSlice{dims: Vec<Slice>}, // not supported
    Index{value: Box<Expr>},
}
#[derive(Copy, Clone, Debug)]
pub enum BoolOp {
    And, Or,
}
#[derive(Copy, Clone, Debug)]
pub enum Operator {
    Add, Sub, Mult, MatMult, Div, Mod, Pow, LShift, RShift, BitOr, BitXor, BitAnd, FloorDiv,
}
#[derive(Copy, Clone, Debug)]
pub enum UnaryOp {
    Invert, Not, UAdd, USub,
}
#[derive(Copy, Clone, Debug)]
pub enum CmpOp {
    Eq, NotEq, Lt, LtE, Gt, GtE, Is, IsNot, In, NotIn, // In and NotIn are not supported
}
#[derive(Debug, Clone)]
pub struct Comprehension{target: Box<Expr>, iter: Box<Expr>, ifs: Vec<Expr>}
#[derive(Debug, Clone)]
pub struct ExceptHandler{etype: Option<Expr>, name: Option<Identifier>, body: Vec<Stmt>, location: String}
#[derive(Debug, Clone)]
pub struct Arguments{args: Vec<Arg>, vararg: Option<Arg>, kwonlyargs: Vec<Arg>, kw_defaults: Vec<Expr>, kwarg: Option<Arg>, defaults: Vec<Expr>}
#[derive(Debug, Clone)]
pub struct Arg{arg: Identifier, annotation: Option<Expr>, location: String}
#[derive(Debug, Clone)]
pub struct Keyword{arg: Option<Identifier>, value: Box<Expr>}


// provides nicer printing of AST than just debug-print
impl Display for Stmt {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), Error> {
        use self::Stmt::*;
        match *self {
            FunctionDef{ref name, ref args, ref body, ref decorator_list, ref returns, ref location} => {
                let body_block = body.iter().map(|st| format!("  {}", st).replace("\n", "\n  ")).collect::<Vec<String>>()[..].join(",\n");
                write!(fmt, "FunctionDef ({}, {:?}, [\n{}], {:?}, {:?} {})", name, args, body_block, decorator_list, returns, location)
                },
            Return{ref value, ref location} => {
                let expr = match *value { None => "".to_string(), Some(ref r) => format!("  {}", r).replace("\n", "\n  ")};
                write!(fmt, "Return ({}, {})", expr, location)
                },
            Assign{ref target, ref value, ref location} => {
                write!(fmt, "Assign ({} =, {}, {})", format!("{}", target).replace("\n", "\n  "), format!("{}", value).replace("\n", "\n  "), location)
                },
            AugAssign{ref target, op, ref value, ref location} => {
                write!(fmt, "AugAssign ({} {:?}, {}, {})", format!("{}", target).replace("\n", "\n  "), op, format!("{}", value).replace("\n", "\n  "), location)
                },
            While{ref test, ref body, ref orelse, ref location} => {
                let body_block = body.iter().map(|st| format!("  {}", st).replace("\n", "\n  ")).collect::<Vec<String>>()[..].join(",\n");
                let else_block = orelse.iter().map(|st| format!("  {}", st).replace("\n", "\n  ")).collect::<Vec<String>>()[..].join(",\n");
                write!(fmt, "While ({}:, [\n{}], [\n{}], {})", format!("{}", test).replace("\n", "\n  "), body_block, else_block, location)
                },
            If{ref test, ref body, ref orelse, ref location} => {
                let body_block = body.iter().map(|st| format!("  {}", st).replace("\n", "\n  ")).collect::<Vec<String>>()[..].join(",\n");
                let else_block = orelse.iter().map(|st| format!("  {}", st).replace("\n", "\n  ")).collect::<Vec<String>>()[..].join(",\n");
                write!(fmt, "If ({}:, [\n{}], [\n{}], {})", format!("{}", test).replace("\n", "\n  "), body_block, else_block, location)
                },
            Global{ref names, ref location} => write!(fmt, "Global ({:?}, {})", names, location),
            Nonlocal{ref names, ref location} => write!(fmt, "Nonlocal ({:?}, {})", names, location),
            Pass{ref location} => write!(fmt, "Pass {})", location),
            Break{ref location} => write!(fmt, "Break {})", location),
            Continue{ref location} => write!(fmt, "Continue {})", location),
            Expr{ref value, ref location} => write!(fmt, "Expr (\n {}, {})", format!("{}", value).replace("\n", "\n  "), location),
        }
    }
}

impl Display for Expr {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), Error> {
        use self::Expr::*;
        match *self {
            BoolOpE{op, ref values, ref location} => write!(fmt, "BoolOpE (\n{},\n{:?}, \n{}, {})", format!("  {}", values[0]).replace("\n", "\n  "), op, format!("  {}", values[1]).replace("\n", "\n  "), location),
            BinOp{ref left, op, ref right, ref location} => write!(fmt, "BinOp (\n{},\n{:?}, \n{}, {})", format!("  {}", left).replace("\n", "\n  "), op, format!("  {}", right).replace("\n", "\n  "), location),
            UnaryOpE{op, ref operand, ref location} => write!(fmt, "UnaryOpE ({:?}, \n{}, {})", op, format!("  {}", operand).replace("\n", "\n  "), location),
            Dict{ref keys, ref values, ref location} => write!(fmt, "Dict ({:?}, {:?}, {})", keys, values, location),
            Compare{ref left, ref ops, ref comparators, ref location} => {
                let comparators_e = comparators.iter().map(|st| format!("  {}", st).replace("\n", "\n  ")).collect::<Vec<String>>()[..].join(",\n");
                write!(fmt, "Compare ({}, {:?}, [\n{}], {})", format!("{}", left).replace("\n", "\n  "), ops, comparators_e, location)
                },
            Call{ref func, ref args, ref keywords, ref location} => {
                let args_e = args.iter().map(|st| format!("  {}", st).replace("\n", "\n  ")).collect::<Vec<String>>()[..].join(",\n");
                write!(fmt, "Call ({}, [\n{}], {:?}, {})", func, args_e, keywords, location)
                },
            Str{ref s, ref location} => write!(fmt, "String ({}, {})", s, location),
            Num{n, ref location} => write!(fmt, "Num ({}, {})", n, location),
            NameConstant{value, ref location} => write!(fmt, "NameConstant ({}, {})", value, location),
            Ellipsis{ref location} => write!(fmt, "Ellipsis ({})", location),
            Name{ref id, ctx, ref location} => write!(fmt, "Name ({}, {:?}, {})", id, ctx, location),
            Subscript{ref value, ref slice, ctx, ref location} => write!(fmt, "Subscript ({}, {:?}, {:?}, {})", format!("{}", value).replace("\n", "\n  "), slice, ctx, location),
            _ => write!(fmt, "DISPLAY NOT IMPLEMENTED"),
        }
    }
}


