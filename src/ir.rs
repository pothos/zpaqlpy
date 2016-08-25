use std::fmt::{Display, Formatter, Error};

#[derive(Debug, Clone)]
pub enum IR {
    Assign{target: IRVar, source: IRVar}, // e.g. M[t7] = 13
    Assign2Op{target: IRVar, val1: IRVar, op: IROp, val2: IRVar}, // e.g. t3 = 2 + H[t9]
    Assign1Op{target: IRVar, uop: IRUOp, source: IRVar}, // e.g. t2 = -t3
    GoTo{label: String},
    Label{label: String},
    Halt, // end ZPAQL execution for the current input byte
    Out{var: IRVar}, // produce pcomp output
    If{cond_var: IRVar, goto_label: String}, // cond_var is false for 0, otherwise true
    IfN{cond_var: IRVar, goto_label: String},
    IfEq{val1: IRVar, val2: IRVar, goto_label: String},
    IfNeq{val1: IRVar, val2: IRVar, goto_label: String},
    Error, // end ZPAQL execution totally through the "wrong opcode" message
    Comment{comment: String},
    // helper meta commands which will be converted to upper commands before they get to the ZPAQL backend
    Block{stmts: Vec<IR>},
    MarkTempVarStart,
    MarkTempVarEnd,
    StoreTempVars{ti: Vec<u8>, stack_pos: u32}, // save variables ti to the stack    (remember to also set st.stack_pos afterwards if needed! - not needed when only Call is comming)
    LoadTempVars{ti: Vec<u8>, stack_pos: u32}, // load ti from the stack (stack_pos+1 is position of saved t1)
    Call{label: String, args: Vec<IRVar>, stack_pos: u32, ret_id: u32},  // will overwrite t1 with return value
    Return{var: Option<IRVar>},
    JumpCode{ret_ids: Vec<u32>, stackend: u32},  // set via st.make_new_return_id(), create jumpers for return ids
    InitialCode{bsp: u32},
}

impl IR {
    pub fn convert(&self) -> IR { // expand meta commands
        match *self {
            IR::InitialCode{bsp} => {
                IR::Block{stmts: vec![
                    IR::Comment{comment: "t255 holds the inital value passed into the A register, first ZPAQL instruction must thus be r=a 255".to_string()},
                    IR::IfN{cond_var: IRVar::Var{varid: 0}, goto_label: "init_code".to_string()}, // basepointer is not set yet, run init code
                    IR::If{cond_var: IRVar::Var{varid: 254}, goto_label: "cont_reading".to_string()}, // proceed with read_b function which was stopped through halt in order to have a new input byte in t255
                    IR::GoTo{label: "call_next".to_string()},
                    IR::Label{label: "init_code".to_string()},
                    IR::Assign{target: IRVar::Var{varid: 0}, source: IRVar::Number{value: bsp}}, // initialize bsp
                    IR::Assign{target: IRVar::Var{varid: 252}, source: IRVar::Var{varid: 0}},  // save globalbsp
                    IR::GoTo{label: "read_b_end~".to_string()}, // define read_b(), which does not use parameters
                    IR::Label{label: "read_b".to_string()},
                    // test if input_c (at t253) is consumed already
                    IR::Assign2Op{target: IRVar::Var{varid: 1}, val1: IRVar::Var{varid: 253}, op: IROp::Eq, val2: IRVar::Number{value: 4294967294}},
                    IR::If{cond_var: IRVar::Var{varid: 1}, goto_label: "do_read_in".to_string()},
                    // was not consumed:
                    IR::Assign{target: IRVar::Var{varid: 255}, source: IRVar::Var{varid: 253}},  // t255 = input_c
                    IR::Assign{target: IRVar::Var{varid: 253}, source: IRVar::Number{value: 4294967294}}, // input_c = -1
                    IR::GoTo{label: "cont_reading".to_string()}, // no halt needed now
                    IR::Label{label: "do_read_in".to_string()},
                    // t254 holds reading state
                    IR::Assign{target: IRVar::Var{varid: 254}, source: IRVar::Number{value: 1}},  // in reading state
                    IR::Halt, // halt to get t255 filled with new input byte, starts execution from the beginning, therefore we have to jump back to cont_reading
                    IR::Label{label: "cont_reading".to_string()},
                    IR::Assign{target: IRVar::Var{varid: 254}, source: IRVar::Number{value: 0}},  // not in reading state
                    IR::Return{var: Some(IRVar::Var{varid: 255})}.convert(),
                    IR::Label{label: "read_b_end~".to_string()}, // end of read_b() function
                ]}
            },
            IR::JumpCode{ref ret_ids, stackend} => {
                let mut stmts = vec![
                    IR::Halt,
                    IR::Label{label: "find_label_ret_id".to_string()},  // expects ret_id to be in t2
                ];
                stmts.push(IR::Assign2Op{target: IRVar::Var{varid: 4}, val1: IRVar::Var{varid: 0}, op: IROp::Gt, val2: IRVar::Number{value: stackend-200} });
                stmts.push(IR::If{cond_var: IRVar::Var{varid: 4}, goto_label: "throw_error".to_string()});
                for ret_id in ret_ids.iter() {
                    stmts.push(IR::IfEq{val1: IRVar::Var{varid: 2}, val2: IRVar::Number{value: *ret_id}, goto_label: format!("return_id_{}", ret_id)});
                }
                stmts.push(IR::Label{label: "throw_error".to_string()});
                stmts.push(IR::Error);
                stmts.push(IR::Halt);
                IR::Block{stmts: stmts}
            },
            IR::Return{ref var} => {
                let mut stmts = vec![];
                match var {
                    &None => {},
                    // t1 = var
                    &Some(ref v) => { stmts.push(IR::Assign{target: IRVar::Var{varid: 1}, source: (*v).clone()}); },
                }
                // load ret_id to t2
                stmts.push(IR::Assign{target: IRVar::Var{varid: 2},
                    source: IRVar::H{index_varid: 0, orig_name: "".to_string()}});
                // t0--
                stmts.push(IR::Assign2Op{target: IRVar::Var{varid: 0}, val1: IRVar::Var{varid: 0}, op: IROp::Sub, val2: IRVar::Number{value: 1}});
                // load old_bsp from t0 to t0
                stmts.push(IR::Assign{target: IRVar::Var{varid: 0},
                    source: IRVar::H{index_varid: 0, orig_name: "".to_string()}});
                stmts.push(IR::GoTo{label: "find_label_ret_id".to_string()});
                IR::Block{stmts: stmts}
            },
            IR::Call{ref label, ref args, stack_pos, ret_id} => {
                // Calling convention:
                // Expects t0 to be the new bsp (i.e. pointer to stack_pos+2), then with new t0: old_bsp in H[t0-1],
                //   ret_id in H[t0], and arg1 in H[t0+1] etc.
                // After return, the result value will be in t1 and t0 will be the restored old_bsp
                let mut stmts = vec![
                    IR::Assign{target: IRVar::Ht{stack_offset: stack_pos+1, local: true, orig_name: "".to_string()},
                        source: IRVar::Var{varid: 0}}, // save old_bsp
                    IR::Comment{comment: "saved bsp, return id:".to_string()},
                    IR::Assign{target: IRVar::Ht{stack_offset: stack_pos+2, local: true, orig_name: "".to_string()},
                        source: IRVar::Number{value: ret_id}}, // set ret_id
                    IR::Comment{comment: "push arguments:".to_string()},
                ];
                for (pos, arg) in args.iter().enumerate() {
                    stmts.push(IR::Assign{target: IRVar::Ht{stack_offset: stack_pos+3+ pos as u32, local: true, orig_name: "".to_string()},
                        source: (*arg).clone()});
                }
                stmts.push(IR::Assign2Op{target: IRVar::Var{varid: 0}, val1: IRVar::Var{varid: 0}, op: IROp::Add, val2: IRVar::Number{value: stack_pos+2}});
                stmts.push(IR::GoTo{label: label.clone()});
                stmts.push(IR::Label{label: format!("return_id_{}", ret_id)});
                IR::Block{stmts: stmts}
            },
            IR::StoreTempVars{ref ti, stack_pos} => {
                let mut stmts = vec![];
                let mut i = 1;
                for vi in ti {
                    stmts.push(IR::Assign{target: IRVar::Ht{stack_offset: stack_pos+i, local: true, orig_name: "".to_string()},
                        source: IRVar::Var{varid: *vi}});
                    i += 1;
                }
                IR::Block{stmts: stmts}
            },
            IR::LoadTempVars{ref ti, stack_pos} => {
                let mut stmts = vec![];
                let mut i = 1;
                for vi in ti {
                    stmts.push(IR::Assign{target: IRVar::Var{varid: *vi},
                        source: IRVar::Ht{stack_offset: stack_pos+i, local: true, orig_name: "".to_string()}});
                    i += 1;
                }
                IR::Block{stmts: stmts}
            },
            _ => self.clone(),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum IRVar {
  Number{value: u32}, // all computations are in Z_4294967296
  Var{varid: u8},  // varid 255, 254, 253, 252 are reserved for input byte, reading state, input_c, globalbsp
  H{index_varid: u8, orig_name: String},
  Ht{stack_offset: u32, local: bool, orig_name: String},  // stack_pos is offset from t0 (local=true) or t252 (local=false)
  M{index_varid: u8},
  Hx{addr: u32},
  Mx{addr: u32},
  VH(Box<IRVar>), // virtual wrappers for arrays on H and M (exist for type information to reduce dynamic checks)
  VM(Box<IRVar>),
}

impl IRVar {
    pub fn tovar(&self) -> IRVar { // expand meta commands
        match *self {
            IRVar::VH(ref x) => { (**x).clone() },
            IRVar::VM(ref x)=> { (**x).clone() },
            _ => self.clone(),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum IROp {
  Add,
  Sub,
  Mult,
  Div,
  Pow,
  LShift,
  RShift,
  Mod,

  BitOr,
  BitXor,
  BitAnd,

  Or,
  And,
  Eq,
  NotEq,
  Lt,
  LtE,
  Gt,
  GtE,
}

#[derive(Debug, Clone, Copy)]
pub enum IRUOp {
  Not,     // (== 0)
  Invert,  // bitwise
  USub,
}

impl Display for IROp {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), Error> {
        use self::IROp::*;
        match *self {
            Add => { write!(fmt, "+") },
            Sub => { write!(fmt, "-") },
            Mult => { write!(fmt, "*") },
            Div => { write!(fmt, "/") },
            Pow => { write!(fmt, "**") },
            Mod => { write!(fmt, "%") },
            LShift => { write!(fmt, "<<") },
            RShift => { write!(fmt, ">>") },
            BitOr => { write!(fmt, "|") },
            BitXor => { write!(fmt, "^") },
            BitAnd => { write!(fmt, "&") },

            Or => { write!(fmt, "or") },
            And => { write!(fmt, "and") },
            Eq => { write!(fmt, "==") },
            NotEq => { write!(fmt, "!=") },
            Lt => { write!(fmt, "<") },
            LtE => { write!(fmt, "<=") },
            Gt => { write!(fmt, ">") },
            GtE => { write!(fmt, ">=") },
        }
    }
}

impl Display for IRUOp {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), Error> {
        use self::IRUOp::*;
        match *self {
            Not => { write!(fmt, "!") },
            Invert => { write!(fmt, "~") },
            USub => { write!(fmt, "-") },
        }
    }
}

impl Display for IR {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), Error> {
        use self::IR::*;
        match *self {
            Assign{ref target, ref source} => {
                write!(fmt, " {} = {}", target, source)
            },
            Assign2Op{ref target, ref val1, op, ref val2} => {
                write!(fmt, " {} = {} {} {}", target, val1, op, val2)
            },
            Assign1Op{ref target, uop, ref source} => {
                write!(fmt, " {} = {} {}", target, uop, source)
            },
            GoTo{ref label} => {
                write!(fmt, " goto {}", label)
            },
            Label{ref label} => {
                write!(fmt, ":{}:", label)
            },
            Halt => {
                write!(fmt, " halt")
            },
            Out{ref var} => {
                write!(fmt, " out {}", var)
            },
            If{ref cond_var, ref goto_label} => {
                write!(fmt, " if {} goto {}", cond_var, goto_label)
            },
            IfN{ref cond_var, ref goto_label} => {
                write!(fmt, " ifN {} goto {}", cond_var, goto_label)
            },
            IfEq{ref val1, ref val2, ref goto_label} => {
                write!(fmt, " ifEq {} {} goto {}", val1, val2, goto_label)
            },
            IfNeq{ref val1, ref val2, ref goto_label} => {
                write!(fmt, " ifNeq {} {} goto {}", val1, val2, goto_label)
            },
            Error => {
                write!(fmt, " error")
            },
            Block{ref stmts} => {
                let block = stmts.iter().map(|st| format!("  {}", st).replace("\n", "\n  ")).collect::<Vec<String>>()[..].join("\n");
                write!(fmt, " {}", block)
            },
            Comment{ref comment} => {
                write!(fmt, " # {}", comment)
            },
            InitialCode{bsp} => { write!(fmt, " InitialCode (bsp: {})", bsp) },
            MarkTempVarStart => { write!(fmt, " MarkTempVarStart") },
            MarkTempVarEnd => { write!(fmt, " MarkTempVarEnd") },
            StoreTempVars{ref ti, stack_pos} => { write!(fmt, " StoreTempVars(t{:?}, stack_pos: {})", ti, stack_pos) },
            LoadTempVars{ref ti, stack_pos} => { write!(fmt, " LoadTempVars(t{:?}, stack_pos: {})", ti, stack_pos) },
            JumpCode{ref ret_ids, stackend} => { write!(fmt, " JumpCode(ret_ids: {:?}, stackend: {})", ret_ids, stackend) },
            Call{ref label, ref args, stack_pos, ret_id} => { write!(fmt, " t1 = {}({}) # stack_pos: {} return id: {}", label, args.iter().map(|v| format!("{}", v)).collect::<Vec<String>>()[..].join(", "), stack_pos, ret_id) },
            Return{ref var} => { write!(fmt, " return {}", match var { &Some(ref v) => format!("{}", v), &None => "".to_string() } ) },
        }
    }
}


impl Display for IRVar {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), Error> {
        use self::IRVar::*;
        match self.tovar() {
            Number{value} => { write!(fmt, "{}", value) },
            Var{varid} => { write!(fmt, "t{}", varid) },
            H{index_varid, ref orig_name} => {
                write!(fmt, "H[t{}]({})", index_varid, orig_name)
            },
            Ht{stack_offset, local, ref orig_name} => {
                write!(fmt, "H[t{}+{}]({})", if local {0}else{252}, stack_offset, orig_name)
            },
            M{index_varid} => { write!(fmt, "M[t{}]", index_varid) },
            Mx{addr} => { write!(fmt, "M[{}]", addr) },
            Hx{addr} => { write!(fmt, "H[{}]", addr) },
            _ => unreachable!(),
        }
    }
}


