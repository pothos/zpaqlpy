#![allow(unused_imports)]
use std::collections::{HashMap};
use std::str::FromStr;
use ast::{Stmt, Expr, ExprContext, Slice, BoolOp, Operator, Keyword, Arg, Arguments, Comprehension, CmpOp, UnaryOp};
use ir::{IR, IRVar, IROp, IRUOp};
use zpaqcfg::ZPAQCfgFile;
use options;

/// evaluate an AST expression to IRVar with the needed instructions, does acquire temporary variables
pub fn evaluate(expr: &Expr, mut st: &mut SymbolTable, optioncfg: &options::Options) -> (Vec<IR>, IRVar) {
    let mut irc = vec![];
    match st.source_line(expr.location()) {
        Some(line) => { if optioncfg.comments { irc.push(IR::Comment{comment: line}); } },
        None => {},
    }
    let var = match expr {
        &Expr::Num{n, location: _} => IRVar::Number{value: n},
        &Expr::Ellipsis{location: _} => IRVar::Number{value: 0},  // is a placeholder like 'pass'
        &Expr::NameConstant{value: n, location: _} => IRVar::Number{value: n},  // False=0, True=1 (but could be everything else than 0)
        &Expr::Name{ref id, ctx, location: _} => {
            match ctx {
                ExprContext::Load | ExprContext::Store => {},
                _ => { error!("only access is supported, del and others not"); panic!("error") },
            }
            let (va, code) = st.get_value(id, optioncfg); irc.extend_from_slice(&code[..]);  // get value from symbol table
            va
        },
        &Expr::BoolOpE{op, ref values, location: _} => {  // boolean operation which selects one of the values
            assert!(values.len() == 2);  // parser builds up a tree with correct precedence, not done here, so length is only 2
            let res = IRVar::Var{varid: st.make_temp_var()};
            let (eval1_irc, e1) = evaluate(&values[0], st, optioncfg);
            irc.extend_from_slice(&eval1_irc[..]);
            let (eval2_irc, e2) = evaluate(&values[1], st, optioncfg);
            irc.extend_from_slice(&eval2_irc[..]);
            st.try_freeing_varid(&e1); st.try_freeing_varid(&e2);
            irc.push(IR::Assign2Op{target: res.clone(), val1: e1, op: match op { BoolOp::And => IROp::And, BoolOp::Or => IROp::Or, }, val2: e2});
            res
        },
        &Expr::UnaryOpE{op, ref operand, location: _} => {
            let res = IRVar::Var{varid: st.make_temp_var()};
            let (eval_irc, val) = evaluate(&(*operand), st, optioncfg);
            irc.extend_from_slice(&eval_irc[..]);
            st.try_freeing_varid(&val);
            match op {
                UnaryOp::Not => { irc.push(IR::Assign1Op{target: res.clone(), uop: IRUOp::Not, source: val}); },
                UnaryOp::Invert => { irc.push(IR::Assign1Op{target: res.clone(), uop: IRUOp::Invert, source: val}); },
                UnaryOp::USub => { irc.push(IR::Assign1Op{target: res.clone(), uop: IRUOp::USub, source: val}); },
                UnaryOp::UAdd => { irc.push(IR::Assign{target: res.clone(), source: val});  }, // just a copy
            }
            res
        },
        &Expr::Compare{ref left, ref ops, ref comparators, location: _} => { // left <ops[0]> comparators[0] <ops[1]> comparators[1] …
            // semantics of a == b == c differs from (a == b) == c, middle operand is split up and the expressions
            // are merged with AND, propagate b to next comparison
            let res = IRVar::Var{varid: st.make_temp_var()};
            let (eval1_irc, mut last) = evaluate(left, st, optioncfg);
            irc.extend_from_slice(&eval1_irc[..]);
            let test_end_label = st.get_new_label("test_cmp_end");
            let label_needed = ops.len() > 1;
            for (op, ref expr) in ops.iter().zip(comparators.iter()) {
                let (eval2_irc, cur_e) = evaluate(expr, st, optioncfg);
                irc.extend_from_slice(&eval2_irc[..]);
                st.try_freeing_varid(&last);
                match *op {  // res = last <op> cur_e
                    CmpOp::Eq | CmpOp::Is => irc.push(IR::Assign2Op{target: res.clone(), val1: last, op: IROp::Eq, val2: cur_e.clone()}),
                    CmpOp::NotEq | CmpOp::IsNot => irc.push(IR::Assign2Op{target: res.clone(), val1: last, op: IROp::NotEq, val2: cur_e.clone()}),
                    CmpOp::Lt => irc.push(IR::Assign2Op{target: res.clone(), val1: last, op: IROp::Lt, val2: cur_e.clone()}),
                    CmpOp::LtE => irc.push(IR::Assign2Op{target: res.clone(), val1: last, op: IROp::LtE, val2: cur_e.clone()}),
                    CmpOp::Gt => irc.push(IR::Assign2Op{target: res.clone(), val1: last, op: IROp::Gt, val2: cur_e.clone()}),
                    CmpOp::GtE => irc.push(IR::Assign2Op{target: res.clone(), val1: last, op: IROp::GtE, val2: cur_e.clone()}),
                    CmpOp::In | CmpOp::NotIn => { error!("in and not in are unsupported"); panic!("error") },
                }
                if label_needed {
                    irc.push(IR::IfN{cond_var: res.clone(), goto_label: test_end_label.clone()});
                }
                last = cur_e;
            }
            st.try_freeing_varid(&last);
            if label_needed{
                irc.push(IR::Label{label: test_end_label});
            }
            res
        },
        &Expr::Call{ref func, ref args, keywords: _, ref location} => {
            if func.as_str() == "out" { // handle special API functions as inline functions
                let (eval_irc, eval_res) = evaluate(&(args[0]), st, optioncfg);
                irc.extend_from_slice(&eval_irc[..]);
                st.try_freeing_varid(&eval_res);
                irc.push(IR::Out{var: eval_res});
                IRVar::Number{value: 0}
            } else if func.as_str() == "error" {
                irc.push(IR::Error);  // libzpaq will fail with "Bad ZPAQL opcode"
                IRVar::Number{value: 4294967295} // dummy
            } else if func.as_str() == "push_b" {
                if args.len() != 1 {
                    error!("push_b() takes exactly one argument: {}", location);
                    panic!("error")
                }
                let (eval_irc, eval_res) = evaluate(&(args[0]), st, optioncfg);
                irc.extend_from_slice(&eval_irc[..]);
                st.try_freeing_varid(&eval_res);
                irc.push(IR::Assign{target: IRVar::Var{varid: 253}, source: eval_res.clone()});  // input_c = arg1
                IRVar::Number{value: 0}
            } else if func.as_str() == "len" {
                if args.len() > 1 {
                    error!("len() only takes on argument: {}", location);
                    panic!("error")
                }
                match &args[0] {
                    &Expr::Name{ref id, ctx: ExprContext::Load, location: _} => {
                        match id.as_str() {
                            "hH" => IRVar::Number{value: 2u32.pow(st.hh as u32)},
                            "hM" => IRVar::Number{value: 2u32.pow(st.hm as u32)},
                            "pH" => IRVar::Number{value: 2u32.pow(st.ph as u32)},
                            "pM" => IRVar::Number{value: 2u32.pow(st.pm as u32)},
                            _ => { error!("len() is only supported for hH, hM, pH, pM"); panic!("error") },
                        } // @TODO: move len_xY here by typechecking for VH/VM or dynamic by 32nd bit (not implemented yet)
                          //        but on the other hand len_xY makes it visible that the array is in xY
                        },
                    _ => { error!("len() is only supported for hH, hM, pH, pM (maybe you need e.g. len_pM after alloc_pM)"); panic!("error") },
                }
            } else if func.as_str() == "array_pM" || func.as_str() == "array_hM" || func.as_str() == "array_pH" || func.as_str() == "array_hH" {
                if args.len() != 1 {
                    error!("array_??() takes exactly one argument: {}", location);
                    panic!("error")
                }
                // Cast not needed as array is already an integer pointer and not a VirtArray object
                // as in the Python code where a cast is needed after a reference is itself retrieved
                // from an array and needs to be converted back.
                // It exists here for the type information by wrapping it in VM/VH
                // which can later be used to spare the dynamic checks.
                let (eval_irc, eval_res) = evaluate(&(args[0]), st, optioncfg);
                irc.extend_from_slice(&eval_irc[..]);
                if func.as_str() == "array_pM" || func.as_str() == "array_hM" {
                    IRVar::VM(Box::new(eval_res))
                } else {
                    IRVar::VH(Box::new(eval_res))
                }
            } else if func.as_str() == "len_pM" || func.as_str() == "len_hM" || func.as_str() == "len_pH" || func.as_str() == "len_hH" {
                let (eval_irc, eval_res) = evaluate(&(args[0]), st, optioncfg);
                irc.extend_from_slice(&eval_irc[..]);
                let addr_t = st.make_temp_var();
                st.try_freeing_varid(&eval_res);
                irc.push(IR::Assign{target: IRVar::Var{varid: addr_t}, source: eval_res});
                match func.as_str() { // gets the length of the dynamic array allocations from their pointer
                    "len_hH" | "len_pH" => {  // point to length entry by -= 2
                        irc.push(IR::Assign2Op{target: IRVar::Var{varid: addr_t}, val1: IRVar::Var{varid: addr_t}, op: IROp::Sub, val2: IRVar::Number{value: 2}});
                        IRVar::H{index_varid: addr_t, orig_name: "".to_string()}
                    },
                    "len_hM" | "len_pM" => {  // code is longer to assemble the four bytes to a 32-bit number again
                        // convert to real pointer from typed-pointer by & ((1<<31)-1)
                        irc.push(IR::Assign2Op{target: IRVar::Var{varid: addr_t}, val1: IRVar::Var{varid: addr_t}, op: IROp::BitAnd, val2: IRVar::Number{value: 2147483647} });
                        // point to begin of length entry by -= 5 and restore u32 from it's bytes
                        irc.push(IR::Assign2Op{target: IRVar::Var{varid: addr_t}, val1: IRVar::Var{varid: addr_t}, op: IROp::Sub, val2: IRVar::Number{value: 5}});
                        let calc_value = IRVar::Var{varid: st.make_temp_var()};
                        // calc_value = M[addr] << 8
                        irc.push(IR::Assign2Op{target: calc_value.clone(), val1: IRVar::M{index_varid: addr_t}, op: IROp::LShift, val2: IRVar::Number{value: 8} });
                        // addr += 1
                        irc.push(IR::Assign2Op{target: IRVar::Var{varid: addr_t}, val1: IRVar::Var{varid: addr_t}, op: IROp::Add, val2: IRVar::Number{value: 1}});
                        // calc_value += M[addr]
                        irc.push(IR::Assign2Op{target: calc_value.clone(), val1: calc_value.clone(), op: IROp::Add, val2: IRVar::M{index_varid: addr_t} });
                        // calc_value <<= 8
                        irc.push(IR::Assign2Op{target: calc_value.clone(), val1: calc_value.clone(), op: IROp::LShift, val2: IRVar::Number{value: 8} });
                        // addr += 1
                        irc.push(IR::Assign2Op{target: IRVar::Var{varid: addr_t}, val1: IRVar::Var{varid: addr_t}, op: IROp::Add, val2: IRVar::Number{value: 1}});
                        // calc_value += M[addr]
                        irc.push(IR::Assign2Op{target: calc_value.clone(), val1: calc_value.clone(), op: IROp::Add, val2: IRVar::M{index_varid: addr_t} });
                        // calc_value <<= 8
                        irc.push(IR::Assign2Op{target: calc_value.clone(), val1: calc_value.clone(), op: IROp::LShift, val2: IRVar::Number{value: 8} });
                        // addr += 1
                        irc.push(IR::Assign2Op{target: IRVar::Var{varid: addr_t}, val1: IRVar::Var{varid: addr_t}, op: IROp::Add, val2: IRVar::Number{value: 1}});
                        // calc_value += M[addr]
                        irc.push(IR::Assign2Op{target: calc_value.clone(), val1: calc_value.clone(), op: IROp::Add, val2: IRVar::M{index_varid: addr_t} });
                        st.try_freeing_varid(&IRVar::Var{varid: addr_t});
                        calc_value
                    },
                    _ => unreachable!(),
                }
            } else if ["alloc_pM", "alloc_hM", "alloc_hH", "alloc_pH", "free_pM", "free_hM", "free_hH", "free_pH"].contains(&(func.as_str())) {
                // pointers in M have 1<<31 added to distinguish from pointers in H
                // So last addressable starting point for any list is 2147483647 == (1<<31)-1
                let func_name = format!("addr_{}", func);  // forward to zpaqlpy function
                let arg = match func.as_str() { // convert to real pointer for free by bit operation & ((1<<31)-1)
                    "free_pM" | "free_hM" => Expr::BinOp{left: Box::new(args[0].clone()), op: Operator::BitAnd, right: Box::new(Expr::Num{n: 2147483647, location: location.clone()}), location: location.clone()},
                    _ => args[0].clone(),
                };
                let (eval_irc, eval_res) = evaluate(&Expr::Call{func: func_name, args: vec![arg], keywords: vec![], location: location.clone()}, st, optioncfg);
                irc.extend_from_slice(&eval_irc[..]);
                match func.as_str() { // convert to typed-pointer by bit operation |= (1<<31)
                    "alloc_pM" | "alloc_hM" => {
                        // eval_res is of type IRVar::Var, so this works
                        irc.push(IR::Assign2Op{target: eval_res.clone(), val1: eval_res.clone(), op: IROp::BitOr, val2: IRVar::Number{value: 2147483648} });
                        IRVar::VM(Box::new(eval_res))
                    },
                    "alloc_pH" | "alloc_hH" => {
                        IRVar::VH(Box::new(eval_res))
                    },
                    _ => { eval_res },
                }
            } else if func.as_str() == "peek_b" {
                assert!(args.is_empty());
                let no_call_label = st.get_new_label("no_read_call");
                irc.push(IR::IfNeq{val1: IRVar::Var{varid: 253}, val2: IRVar::Number{value: 4294967294}, goto_label: no_call_label.clone()});
                irc.push(IR::StoreTempVars{ti: st.live_ids.clone(), stack_pos: st.stack_pos});
                irc.push(IR::Call{label: "read_b".to_string(), args: vec![], stack_pos: st.stack_pos + (st.live_ids.len() as u32), ret_id: st.make_new_return_id() });
                irc.push(IR::Assign{target: IRVar::Var{varid: 253}, source: IRVar::Var{varid: 1}});  // input_c = read_b()
                irc.push(IR::LoadTempVars{ti: st.live_ids.clone(), stack_pos: st.stack_pos});
                irc.push(IR::Label{label: no_call_label});
                IRVar::Var{varid: 253}
            } else { // normal function call
                let mut arguments: Vec<IRVar> = vec![];
                let previous_live_ids = st.live_ids.clone();
                for arg in args {
                    let (eval_irc, eval_res) = evaluate(arg, st, optioncfg);
                    irc.extend_from_slice(&eval_irc[..]);
                    arguments.push(eval_res);
                }
                let res = IRVar::Var{varid: st.make_temp_var()};
                irc.push(IR::StoreTempVars{ti: previous_live_ids.clone(), stack_pos: st.stack_pos});
                // reserve amout of stack variables as StoreTempVars has used them
                irc.push(IR::Call{label: func.clone(), args: arguments.clone(), stack_pos: st.stack_pos + (previous_live_ids.len() as u32), ret_id: st.make_new_return_id() });  // @TODO: use fn-table
                irc.push(IR::Assign{target: res.clone(), source: IRVar::Var{varid: 1}});
                irc.push(IR::LoadTempVars{ti: previous_live_ids, stack_pos: st.stack_pos});
                for argument in arguments {
                    st.try_freeing_varid(&argument);
                }
                res
            }
        },
        &Expr::Subscript{ref value, ref slice, ctx: ExprContext::Load, location: _} => {  // access an array element
            let irvar = match (&(**value), &(**slice)) {
                 (&Expr::Name{ref id, ctx: ExprContext::Load, location: _}, &Slice::Index{value: ref indexvalue}) => {
                    match id.as_str() {
                        "hH" | "pH" => {
                            let (eval_irc, ind_var) = evaluate(&(**indexvalue), &mut st, optioncfg);
                            irc.extend_from_slice(&eval_irc[..]);
                            let ind = match ind_var.tovar() {
                                IRVar::Var{varid} => { varid },
                                _ => {
                                    let i = st.make_temp_var();
                                    irc.push(IR::Assign{target: IRVar::Var{varid: i}, source: ind_var.clone()});
                                    st.try_freeing_varid(&ind_var);
                                    i
                                },
                            };
                            Some(IRVar::H{index_varid: ind, orig_name: "".to_string()})
                        },
                        "hM" | "pM" => {
                            let (eval_irc, ind_var) = evaluate(&(**indexvalue), &mut st, optioncfg);
                            irc.extend_from_slice(&eval_irc[..]);
                            let ind = match ind_var.tovar() {
                                IRVar::Var{varid} => { varid },
                                _ => {
                                    let i = st.make_temp_var();
                                    irc.push(IR::Assign{target: IRVar::Var{varid: i}, source: ind_var.clone()});
                                    st.try_freeing_varid(&ind_var);
                                    i
                                },
                            };
                            Some(IRVar::M{index_varid: ind})
                        },
                        _ => { None }, // next outer match gets executed
                    }
                 },
                 _ => { None }, // outer match must be used as case above also should be handled by it
             };
            if irvar.is_some() { irvar.unwrap() } else {
              match (&(**value), &(**slice)) {
                 (_, &Slice::Index{value: ref indexvalue}) => {  // is index access on a allocated array
                    let (eval_addrirc, eval_addr) = evaluate(&(**value), &mut st, optioncfg);
                    irc.extend_from_slice(&eval_addrirc[..]);
                    let (eval_irc, ind_var) = evaluate(&(**indexvalue), &mut st, optioncfg);
                    irc.extend_from_slice(&eval_irc[..]);
                    let t_id = st.make_temp_var();
                    st.try_freeing_varid(&eval_addr);
                    let t = IRVar::Var{varid: t_id};
                    match eval_addr {
                        IRVar::VH(_) => {  // array in H
                            // t = eval_addr + ind_var
                            irc.push(IR::Assign2Op{target: t.clone(), val1: eval_addr.clone(), op: IROp::Add, val2: ind_var.clone() });
                            // t = H[t]
                            irc.push(IR::Assign{target: t.clone(), source: IRVar::H{index_varid: t_id, orig_name: "".to_string() } });
                        },
                        IRVar::VM(_) => {  // array in M
                            // t = eval_addr & ((1<<31)-1)
                            irc.push(IR::Assign2Op{target: t.clone(), val1: eval_addr.clone(), op: IROp::BitAnd, val2: IRVar::Number{value: 2147483647} });
                            // t = t + ind_var
                            irc.push(IR::Assign2Op{target: t.clone(), val1: t.clone(), op: IROp::Add, val2: ind_var.clone() });
                            // t = M[t]
                            irc.push(IR::Assign{target: t.clone(), source: IRVar::M{index_varid: t_id } });
                        },
                        _ => {  // unclear which array as no type information was available in this scope, test for bit 1<<31
                            // decide on bit 32 whether to access M or H with index eval_addr&((1<<31)-1) + ind_var
                            irc.push(IR::Assign2Op{target: t.clone(), val1: eval_addr.clone(), op: IROp::RShift, val2: IRVar::Number{value: 31} });
                            let m_label = st.get_new_label("isM");
                            let done_label = st.get_new_label("MorHdone");
                            irc.push(IR::If{cond_var: t.clone(), goto_label: m_label.clone()});
                            // t = eval_addr + ind_var
                            irc.push(IR::Assign2Op{target: t.clone(), val1: eval_addr.clone(), op: IROp::Add, val2: ind_var.clone() });
                            // t = H[t]
                            irc.push(IR::Assign{target: t.clone(), source: IRVar::H{index_varid: t_id, orig_name: "".to_string() } });
                            irc.push(IR::GoTo{label: done_label.clone()});
                            irc.push(IR::Label{label: m_label});
                            // t = eval_addr & ((1<<31)-1)
                            irc.push(IR::Assign2Op{target: t.clone(), val1: eval_addr.clone(), op: IROp::BitAnd, val2: IRVar::Number{value: 2147483647} });
                            // t = t + ind_var
                            irc.push(IR::Assign2Op{target: t.clone(), val1: t.clone(), op: IROp::Add, val2: ind_var.clone() });
                            // t = M[t]
                            irc.push(IR::Assign{target: t.clone(), source: IRVar::M{index_varid: t_id } });
                            irc.push(IR::Label{label: done_label});
                        },
                    }
                    t
                 },
                 _ => { error!("only array single-element access of the form array[expr] is supported"); panic!("error"); }
              }
            }
        },
        &Expr::Subscript{value: _, slice: _, ctx: _, location: _} => {
            error!("only array single-element access of the form array[expr] is supported, del and others not"); panic!("error")
        },
        &Expr::BinOp{ref left, op, ref right, location: _} => {  // evaluate left <op> right
            let res = IRVar::Var{varid: st.make_temp_var()};
            let (eval1_irc, e1) = evaluate(&(*left), st, optioncfg);
            irc.extend_from_slice(&eval1_irc[..]);
            let (eval2_irc, e2) = evaluate(&(*right), st, optioncfg);
            irc.extend_from_slice(&eval2_irc[..]);
            irc.push(IR::Assign2Op{target: res.clone(), val1: e1.clone(), val2: e2.clone(), op:
                match op {
                    Operator::Add => IROp::Add,
                    Operator::Sub => IROp::Sub,
                    Operator::Mult => IROp::Mult,
                    Operator::MatMult => { error!("a @ b not supported"); panic!("error") },
                    Operator::Div | Operator::FloorDiv => IROp::Div,  // there are only integer divisions because there are no floats
                    Operator::Mod => IROp::Mod,
                    Operator::Pow => IROp::Pow,
                    Operator::LShift => IROp::LShift,
                    Operator::RShift => IROp::RShift,
                    Operator::BitOr => IROp::BitOr,
                    Operator::BitXor => IROp::BitXor,
                    Operator::BitAnd => IROp::BitAnd,
                }
            });
            st.try_freeing_varid(&e1);  st.try_freeing_varid(&e2);
            res
        },
        &Expr::Dict{keys: _, values: _, location: _} => {
            error!("dicts are not supported"); panic!("error")
        },
        &Expr::Str{s: _, location: _} => {
            warn!("strings are ignored"); IRVar::Number{value: 0}
        },
        &Expr::Attribute{value: _, attr: _, ctx: _, location: _} => {
            error!(".attributes are not supported"); panic!("error")
        },
        &Expr::Starred{value: _, ctx: _, location: _} => {
            error!("*expr is not supported"); panic!("error")
        },
        &Expr::List{elts: _, ctx: _, location: _} => {
            error!("lists are not supported"); panic!("error")
        },
        &Expr::Tuple{elts: _, ctx: _, location: _} => {
            error!("tuples are not supported"); panic!("error")
        },
    };
    (irc, var)
}

/// compile AST to IR, used recursively
pub fn traverse(tree: &[Stmt], mut st: &mut SymbolTable, optioncfg: &options::Options) -> Vec<IR> {
    let mut irc = vec![];
    // optimisation for initialising data with arrays on M with var[0]=7; var[1]=2; var[2]=4  (further optimised in post_zpaql pass)
    let mut temp_pair_vm: Option<(IRVar, IRVar, u8)> = None;
    let mut temp_pair_vh: Option<(IRVar, IRVar, u8)> = None;  // same on H
    for node in tree {
        let mut new_temp_pair_vm: Option<(IRVar, IRVar, u8)> = None;
        let mut new_temp_pair_vh: Option<(IRVar, IRVar, u8)> = None;
        match st.source_line(node.location()) {
            Some(line) => { if optioncfg.comments { irc.push(IR::Comment{comment: line}); } } // write current line of input source as comment
            None => {}
        }
        match node {
            &Stmt::Pass{location: _} => { },  // nothing to do
            &Stmt::AugAssign{ref target, op, ref value, location: _} => {   //   x += y; x *= y etc
                let (eval_irc, val_var) = evaluate(&(**value), &mut st, optioncfg);
                irc.extend_from_slice(&eval_irc[..]);
                match **target {
                    Expr::Name{ref id, ctx: ExprContext::Store, location: _} => {
                        let target_var = if st.symbols.contains_key(id) {
                            match st.symbols.get(id).unwrap().tovar() {
                                var @ IRVar::H{index_varid: _, orig_name: _} => var,
                                var @ IRVar::Ht{stack_offset: _, local: _, orig_name: _} => var,
                                var @ IRVar::Hx{addr: _} => var,
                                // change if variables on R are used as local variables instead of the stack
                                _ => { error!("can not assign to non-array (i.e. (global/local) variable) element of symbol table"); panic!("error") }
                            }
                        } else {
                            error!("variable {} not found", id); panic!("error")
                        };  // target_var = target_var <op> val_var
                        irc.push(IR::Assign2Op{target: target_var.clone(), val1: target_var, op: match op {
                            Operator::Add => IROp::Add, Operator::Sub => IROp::Sub, Operator::Mult => IROp::Mult, Operator::FloorDiv | Operator::Div => IROp::Div,
                            Operator::Mod => IROp::Mod, Operator::Pow => IROp::Pow, Operator::LShift => IROp::LShift, Operator::RShift => IROp::RShift,
                            Operator::BitOr => IROp::BitOr, Operator::BitXor => IROp::BitXor, Operator::BitAnd => IROp::BitAnd,
                            x => { error!("AugAssign operator {:?} not supported", x); panic!() }
                        }, val2: val_var.clone()});
                    },
                    Expr::Subscript{ref value, ref slice, ctx: ExprContext::Store, location: _} => {
                        match (&(**value), &(**slice)) {
                             (&Expr::Name{ref id, ctx: ExprContext::Store, location: _}, &Slice::Index{value: ref indexvalue}) => {
                                match id.as_str() {
                                    "hH" | "pH" => {
                                        let (eval_irc, ind_var) = evaluate(&(**indexvalue), &mut st, optioncfg);
                                        irc.extend_from_slice(&eval_irc[..]);
                                        let ind = match ind_var.tovar() {
                                            IRVar::Var{varid} => { varid },
                                            _ => {
                                                let i = st.make_temp_var();
                                                irc.push(IR::Assign{target: IRVar::Var{varid: i}, source: ind_var.clone()});
                                                i
                                            },
                                        };
                                        let target_var = IRVar::H{index_varid: ind, orig_name: "".to_string()};  // H[ind_var]
                                        // target_var = target_var <op> val_var
                                        irc.push(IR::Assign2Op{target: target_var.clone(), val1: target_var.clone(), op: match op {
                                            Operator::Add => IROp::Add, Operator::Sub => IROp::Sub, Operator::Mult => IROp::Mult, Operator::FloorDiv | Operator::Div => IROp::Div,
                                            Operator::Mod => IROp::Mod, Operator::Pow => IROp::Pow, Operator::LShift => IROp::LShift, Operator::RShift => IROp::RShift,
                                            Operator::BitOr => IROp::BitOr, Operator::BitXor => IROp::BitXor, Operator::BitAnd => IROp::BitAnd,
                                            x => { error!("AugAssign operator {:?} not supported", x); panic!() }
                                        }, val2: val_var.clone()});
                                        st.try_freeing_varid(&ind_var);  st.try_freeing_varid(&target_var);
                                        st.try_freeing_varid(&val_var);; continue;  // jump over next match and last statement
                                    },
                                    "hM" | "pM" => {
                                        let (eval_irc, ind_var) = evaluate(&(**indexvalue), &mut st, optioncfg);
                                        irc.extend_from_slice(&eval_irc[..]);
                                        let ind = match ind_var.tovar() {
                                            IRVar::Var{varid} => { varid },
                                            _ => {
                                                let i = st.make_temp_var();
                                                irc.push(IR::Assign{target: IRVar::Var{varid: i}, source: ind_var.clone()});
                                                i
                                            },
                                        };
                                        let target_var = IRVar::M{index_varid: ind};
                                        irc.push(IR::Assign2Op{target: target_var.clone(), val1: target_var.clone(), op: match op {
                                            Operator::Add => IROp::Add, Operator::Sub => IROp::Sub, Operator::Mult => IROp::Mult, Operator::FloorDiv | Operator::Div => IROp::Div,
                                            Operator::Mod => IROp::Mod, Operator::Pow => IROp::Pow, Operator::LShift => IROp::LShift, Operator::RShift => IROp::RShift,
                                            Operator::BitOr => IROp::BitOr, Operator::BitXor => IROp::BitXor, Operator::BitAnd => IROp::BitAnd,
                                            x => { error!("AugAssign operator {:?} not supported", x); panic!() }
                                        }, val2: val_var.clone()});
                                        st.try_freeing_varid(&ind_var);  st.try_freeing_varid(&target_var);
                                        st.try_freeing_varid(&val_var);; continue;  // jump over next match and last statement
                                    },
                                    _ => { },  // continue with outer match
                                }
                             },
                             _ => { }, // continue with outer match
                        }
                        match (&(**value), &(**slice)) {   // value[slice] += val_var
                             (_, &Slice::Index{value: ref indexvalue}) => {
                                // augassignment version
                                let (eval_addrirc, eval_addr) = evaluate(&(**value), &mut st, optioncfg);
                                irc.extend_from_slice(&eval_addrirc[..]);
                                let (eval_irc, ind_var) = evaluate(&(**indexvalue), &mut st, optioncfg);
                                irc.extend_from_slice(&eval_irc[..]);
                                let t_id = st.make_temp_var();
                                let t = IRVar::Var{varid: t_id};
                                match eval_addr {
                                    IRVar::VH(_) => {  // H[eval_addr+ind_var] <op>= val_var
                                        // t = eval_addr + ind_var
                                        irc.push(IR::Assign2Op{target: t.clone(), val1: eval_addr.clone(), op: IROp::Add, val2: ind_var.clone() });
                                        // H[t] = H[t] <op> val_var
                                        irc.push(IR::Assign2Op{target: IRVar::H{index_varid: t_id, orig_name: "".to_string() }, val1: IRVar::H{index_varid: t_id, orig_name: "".to_string() }, op: match op {
                                            Operator::Add => IROp::Add, Operator::Sub => IROp::Sub, Operator::Mult => IROp::Mult, Operator::FloorDiv | Operator::Div => IROp::Div,
                                            Operator::Mod => IROp::Mod, Operator::Pow => IROp::Pow, Operator::LShift => IROp::LShift, Operator::RShift => IROp::RShift,
                                            Operator::BitOr => IROp::BitOr, Operator::BitXor => IROp::BitXor, Operator::BitAnd => IROp::BitAnd,
                                            x => { error!("AugAssign operator {:?} not supported", x); panic!() }
                                        }, val2: val_var.clone()});
                                    },
                                    IRVar::VM(_) => {  // M[eval_addr&((1<<31)-1) + ind_var] += val_var
                                        // t = eval_addr & ((1<<31)-1)
                                        irc.push(IR::Assign2Op{target: t.clone(), val1: eval_addr.clone(), op: IROp::BitAnd, val2: IRVar::Number{value: 2147483647} });
                                        // t = t + ind_var
                                        irc.push(IR::Assign2Op{target: t.clone(), val1: t.clone(), op: IROp::Add, val2: ind_var.clone() });
                                        // M[t] = M[t] <op> val_var
                                        irc.push(IR::Assign2Op{target: IRVar::M{index_varid: t_id }, val1: IRVar::M{index_varid: t_id }, op: match op {
                                            Operator::Add => IROp::Add, Operator::Sub => IROp::Sub, Operator::Mult => IROp::Mult, Operator::FloorDiv | Operator::Div => IROp::Div,
                                            Operator::Mod => IROp::Mod, Operator::Pow => IROp::Pow, Operator::LShift => IROp::LShift, Operator::RShift => IROp::RShift,
                                            Operator::BitOr => IROp::BitOr, Operator::BitXor => IROp::BitXor, Operator::BitAnd => IROp::BitAnd,
                                            x => { error!("AugAssign operator {:?} not supported", x); panic!() }
                                        }, val2: val_var.clone()});
                                    },
                                    _ => {  // no type information was available in this scope
                                        // decide on bit 32 whether to access M or H with index eval_addr&((1<<31)-1) + ind_var
                                        irc.push(IR::Assign2Op{target: t.clone(), val1: eval_addr.clone(), op: IROp::RShift, val2: IRVar::Number{value: 31} });
                                        let m_label = st.get_new_label("isM_augassign");
                                        let done_label = st.get_new_label("MorHdone_augassign");
                                        irc.push(IR::If{cond_var: t.clone(), goto_label: m_label.clone()});
                                        // t = eval_addr + ind_var
                                        irc.push(IR::Assign2Op{target: t.clone(), val1: eval_addr.clone(), op: IROp::Add, val2: ind_var.clone() });
                                        // H[t] = H[t] <op> val_var
                                        irc.push(IR::Assign2Op{target: IRVar::H{index_varid: t_id, orig_name: "".to_string() }, val1: IRVar::H{index_varid: t_id, orig_name: "".to_string() }, op: match op {
                                            Operator::Add => IROp::Add, Operator::Sub => IROp::Sub, Operator::Mult => IROp::Mult, Operator::FloorDiv | Operator::Div => IROp::Div,
                                            Operator::Mod => IROp::Mod, Operator::Pow => IROp::Pow, Operator::LShift => IROp::LShift, Operator::RShift => IROp::RShift,
                                            Operator::BitOr => IROp::BitOr, Operator::BitXor => IROp::BitXor, Operator::BitAnd => IROp::BitAnd,
                                            x => { error!("AugAssign operator {:?} not supported", x); panic!() }
                                        }, val2: val_var.clone()});
                                        irc.push(IR::GoTo{label: done_label.clone()});
                                        irc.push(IR::Label{label: m_label});
                                        // t = eval_addr & ((1<<31)-1)
                                        irc.push(IR::Assign2Op{target: t.clone(), val1: eval_addr.clone(), op: IROp::BitAnd, val2: IRVar::Number{value: 2147483647} });
                                        // t = t + ind_var
                                        irc.push(IR::Assign2Op{target: t.clone(), val1: t.clone(), op: IROp::Add, val2: ind_var.clone() });
                                        // M[t] = M[t] <op> val_var
                                        irc.push(IR::Assign2Op{target: IRVar::M{index_varid: t_id }, val1: IRVar::M{index_varid: t_id }, op: match op {
                                            Operator::Add => IROp::Add, Operator::Sub => IROp::Sub, Operator::Mult => IROp::Mult, Operator::FloorDiv | Operator::Div => IROp::Div,
                                            Operator::Mod => IROp::Mod, Operator::Pow => IROp::Pow, Operator::LShift => IROp::LShift, Operator::RShift => IROp::RShift,
                                            Operator::BitOr => IROp::BitOr, Operator::BitXor => IROp::BitXor, Operator::BitAnd => IROp::BitAnd,
                                            x => { error!("AugAssign operator {:?} not supported", x); panic!() }
                                        }, val2: val_var.clone()});
                                        irc.push(IR::Label{label: done_label});
                                    },
                                }
                                st.try_freeing_varid(&IRVar::Var{varid: t_id});
                                st.try_freeing_varid(&ind_var);  st.try_freeing_varid(&eval_addr);
                             },
                             _ => { error!("assignment is only allowed to var or array[expr] (no slices)"); panic!("error"); }
                        }
                    },
                    _ => { error!("assignment is only allowed to var or array[expr]"); panic!("error"); }
                }
                st.try_freeing_varid(&val_var);
            },
            &Stmt::Assign{ref target, ref value, location: _} => {   // x = y
                let (eval_irc, val_var) = evaluate(&(**value), &mut st, optioncfg);
                irc.extend_from_slice(&eval_irc[..]);
                match **target {
                    Expr::Name{ref id, ctx: ExprContext::Store, location: _} => {
                        let target_var = if st.symbols.contains_key(id) {
                            match st.symbols.get(id).unwrap().tovar() {
                                var @ IRVar::H{index_varid: _, orig_name: _} => var,
                                var @ IRVar::Ht{stack_offset: _, local: _, orig_name: _} => var,
                                var @ IRVar::Hx{addr: _} => var,
                                // change if variables can be in R
                                _ => { error!("can not assign to non-array (i.e. no local/global variable) element of symbol table"); panic!("error") }
                            }
                        } else {
                            let v = IRVar::Ht{stack_offset: st.make_stack_var(), local: true, orig_name: id.clone()};
                            st.symbols.insert(id.clone(), v.clone());
                            v
                        };
                        st.symbols.insert(id.clone(), match val_var {  // set type information
                                IRVar::VM(_) => IRVar::VM(Box::new(target_var.clone())),
                                IRVar::VH(_) => IRVar::VH(Box::new(target_var.clone())),
                                _ => target_var.clone() } );
                        irc.push(IR::Assign{target: target_var, source: val_var.clone()});
                    },
                    Expr::Subscript{ref value, ref slice, ctx: ExprContext::Store, location: _} => {  // value[slice] = val_var
                        match (&(**value), &(**slice)) {
                             (&Expr::Name{ref id, ctx: ExprContext::Store, location: _}, &Slice::Index{value: ref indexvalue}) => {
                                match id.as_str() {
                                    "hH" | "pH" => {
                                        let (eval_irc, ind_var) = evaluate(&(**indexvalue), &mut st, optioncfg);
                                        irc.extend_from_slice(&eval_irc[..]);
                                        let ind = match ind_var.tovar() {
                                            IRVar::Var{varid} => { varid },
                                            _ => {
                                                let i = st.make_temp_var();
                                                irc.push(IR::Assign{target: IRVar::Var{varid: i}, source: ind_var.clone()});
                                                i
                                            },
                                        }; // H[ind_var] = val_var
                                        irc.push(IR::Assign{target: IRVar::H{index_varid: ind, orig_name: "".to_string()}, source: val_var.clone()});
                                        st.try_freeing_varid(&ind_var);  st.try_freeing_varid(&IRVar::Var{varid: ind});
                                        st.try_freeing_varid(&val_var); continue; // jump over next match and last statement
                                    },
                                    "hM" | "pM" => {
                                        let (eval_irc, ind_var) = evaluate(&(**indexvalue), &mut st, optioncfg);
                                        irc.extend_from_slice(&eval_irc[..]);
                                        let ind = match ind_var.tovar() {
                                            IRVar::Var{varid} => { varid },
                                            _ => {
                                                let i = st.make_temp_var();
                                                irc.push(IR::Assign{target: IRVar::Var{varid: i}, source: ind_var.clone()});
                                                i
                                            },
                                        };
                                        irc.push(IR::Assign{target: IRVar::M{index_varid: ind}, source: val_var.clone()});
                                        st.try_freeing_varid(&ind_var);  st.try_freeing_varid(&IRVar::Var{varid: ind});
                                        st.try_freeing_varid(&val_var); continue; // jump over next match and last statement
                                    },
                                    _ => { }  // next match will be executed
                                }
                            },
                            _ => { }  // next match will be executed
                         }
                         match (&(**value), &(**slice)) {  // assignment on an element of a dynamically allocated array
                             (_, &Slice::Index{value: ref indexvalue}) => {   // value[indexvalue] = val_var
                                // assignment version
                                let (eval_addrirc, eval_addr) = evaluate(&(**value), &mut st, optioncfg);
                                irc.extend_from_slice(&eval_addrirc[..]);
                                let (eval_irc, ind_var) = evaluate(&(**indexvalue), &mut st, optioncfg);
                                irc.extend_from_slice(&eval_irc[..]);
                                let t_id = st.make_temp_var();
                                let t = IRVar::Var{varid: t_id};
                                match eval_addr {
                                    IRVar::VH(_) => {  // type information is available
                                        let idx: u32 = match ind_var { IRVar::Number{value} => (value as i64 -1i64) as u32, _ => -1i32 as u32 };
                                        match temp_pair_vh {
                                            Some((ref teval_addr, IRVar::Number{value}, tt_id)) if teval_addr == &eval_addr && value == idx && tt_id == t_id => {
                                                // t = t + 1
                                                irc.push(IR::Assign2Op{target: t.clone(), val1: t.clone(), op: IROp::Add, val2: IRVar::Number{value: 1} });
                                                // H[t] = val_var
                                                irc.push(IR::Assign{target: IRVar::H{index_varid: t_id, orig_name: "".to_string() }, source: val_var.clone() });
                                            },
                                            Some((_, _, _))
                                            | None => {
                                                // t = eval_addr + ind_var
                                                irc.push(IR::Assign2Op{target: t.clone(), val1: eval_addr.clone(), op: IROp::Add, val2: ind_var.clone() });
                                                // H[t] = val_var
                                                irc.push(IR::Assign{target: IRVar::H{index_varid: t_id, orig_name: "".to_string() }, source: val_var.clone() });
                                            },
                                        }
                                        new_temp_pair_vh = Some((eval_addr.clone(), ind_var.clone(), t_id));  // track last values for optimisations
                                    },
                                    IRVar::VM(_) => {
                                        let idx: u32 = match ind_var { IRVar::Number{value} => (value as i64 -1i64) as u32, _ => -1i32 as u32 };
                                        match temp_pair_vm {
                                            Some((ref teval_addr, IRVar::Number{value}, tt_id)) if teval_addr == &eval_addr && value == idx && tt_id == t_id => {
                                                // t = t + 1
                                                irc.push(IR::Assign2Op{target: t.clone(), val1: t.clone(), op: IROp::Add, val2: IRVar::Number{value: 1} });
                                                // M[t] = val_var
                                                irc.push(IR::Assign{target: IRVar::M{index_varid: t_id }, source: val_var.clone() });
                                            },
                                            Some((_, _, _))
                                            | None => {
                                                // t = eval_addr & ((1<<31)-1)
                                                irc.push(IR::Assign2Op{target: t.clone(), val1: eval_addr.clone(), op: IROp::BitAnd, val2: IRVar::Number{value: 2147483647} });
                                                // t = t + ind_var
                                                irc.push(IR::Assign2Op{target: t.clone(), val1: t.clone(), op: IROp::Add, val2: ind_var.clone() });
                                                // M[t] = val_var
                                                irc.push(IR::Assign{target: IRVar::M{index_varid: t_id }, source: val_var.clone() });
                                            },
                                        }
                                        new_temp_pair_vm = Some((eval_addr.clone(), ind_var.clone(), t_id));  // track last values for optimisations
                                    },
                                    _ => {
                                        // decide on bit 32 whether to access M or H with index eval_addr&((1<<31)-1) + ind_var
                                        irc.push(IR::Assign2Op{target: t.clone(), val1: eval_addr.clone(), op: IROp::RShift, val2: IRVar::Number{value: 31} });
                                        let m_label = st.get_new_label("isM_assign");
                                        let done_label = st.get_new_label("MorHdone_assign");
                                        irc.push(IR::If{cond_var: t.clone(), goto_label: m_label.clone()});
                                        // t = eval_addr + ind_var
                                        irc.push(IR::Assign2Op{target: t.clone(), val1: eval_addr.clone(), op: IROp::Add, val2: ind_var.clone() });
                                        // H[t] = val_var
                                        irc.push(IR::Assign{target: IRVar::H{index_varid: t_id, orig_name: "".to_string() }, source: val_var.clone() });
                                        irc.push(IR::GoTo{label: done_label.clone()});
                                        irc.push(IR::Label{label: m_label});
                                        // t = eval_addr & ((1<<31)-1)
                                        irc.push(IR::Assign2Op{target: t.clone(), val1: eval_addr.clone(), op: IROp::BitAnd, val2: IRVar::Number{value: 2147483647} });
                                        // t = t + ind_var
                                        irc.push(IR::Assign2Op{target: t.clone(), val1: t.clone(), op: IROp::Add, val2: ind_var.clone() });
                                        // M[t] = val_var
                                        irc.push(IR::Assign{target: IRVar::M{index_varid: t_id }, source: val_var.clone() });
                                        irc.push(IR::Label{label: done_label});
                                    },
                                }
                                st.try_freeing_varid(&IRVar::Var{varid: t_id});
                                st.try_freeing_varid(&ind_var);  st.try_freeing_varid(&eval_addr);
                             },
                             _ => { error!("assignment is only allowed to var or array[expr] (no slices)"); panic!("error"); },
                        }
                    },
                    _ => { error!("assignment is only allowed to var or array[expr]"); panic!("error"); }
                }
                st.try_freeing_varid(&val_var);
            },
            &Stmt::FunctionDef{ref name, ref args, ref body, decorator_list: _, returns: _, location: _} => {
                if (name == "pcomp" || name == "hcomp") && body.len() == 1 {
                    match body[0] {
                        Stmt::Pass{location: _} => { return vec![]; }, // def pcomp(): pass means that pcomp is empty, so the whole generated code will just be empty
                        _ => {},
                    }
                }
                // call convention:
                // expects t0 to be the new bsp, old_bsp in H[t0-1]
                // (to be restored when returning, and t1 will contain the result),
                // ret_id in H[t0], and arg1 in H[t0+1] etc
                st.push(); // enter new local scope
                let mut endlabel = name.clone(); endlabel.push_str("_end~");
                irc.push(IR::GoTo{label: endlabel.clone()});
                irc.push(IR::Label{label: name.clone()});
                irc.push(IR::MarkTempVarStart);  // inserted for possible lifetime optimizations in post_ir pass
                for arg in args {
                    let o_i = st.make_stack_var();  // expect argument variables to be on the stack
                    st.symbols.insert(arg.clone(), IRVar::Ht{stack_offset: o_i, local: true, orig_name: arg.clone()});
                    if optioncfg.comments {
                        irc.push(IR::Comment{comment: format!("Arg {} at t0 + {}", arg, o_i)});
                    }
                }
                irc.push(IR::Block{stmts: traverse(body, st, optioncfg)});
                if optioncfg.comments {
                    irc.push(IR::Comment{comment: "insert return as it might not be done by the function:".to_string()});
                }
                irc.push(IR::Return{var: None});
                irc.push(IR::MarkTempVarEnd);
                irc.push(IR::Label{label: endlabel});
                st.pop();
            },
            &Stmt::Expr{ref value, location: _} => {  // expression as statement, return values are not used
                let (eval_irc, irvar) = evaluate(&(**value), &mut st, optioncfg);
                irc.extend_from_slice(&eval_irc[..]);
                st.try_freeing_varid(&irvar);
            },
            &Stmt::Global{ref names, location: _} => {  // give references to global variables into the symbol table so that assignmets do not create new local values
                // gets hardcoded with first bsp instead of fetching the valid bsp from stack and compute the number
                // t_x = global_t0+stack_pos (global_t0 is t252)
                for name in names {
                    match st.previous[0].get(name) {
                        Some(&IRVar::Hx{addr}) => {  // not generated like that but could happen one day
                            if optioncfg.comments {
                                irc.push(IR::Comment{comment: format!("Global {} via H[{}]", name, addr)});
                            }
                            st.symbols.insert(name.clone(), IRVar::Hx{addr: addr});
                        },
                        Some(&IRVar::Ht{stack_offset, local: true, orig_name: _}) => {  // appreas local for global level
                            if optioncfg.fixed_global_access {
                                let addr = st.bsp + stack_offset;  // calculate access index in H of globals each time
                                if optioncfg.comments {
                                    irc.push(IR::Comment{comment: format!("Global {} via H[{}]", name, addr)});
                                }
                                st.symbols.insert(name.clone(), IRVar::Hx{addr: addr});
                                // semi: calc once from total position
                                // semi // irc.push(IR::Assign{target: IRVar::Var{varid: v}, source: IRVar::Number{value: st.bsp + o_i.unwrap()}});
                            } else {
                                //let v = st.make_temp_var();  // calculate once from offset
                                if optioncfg.comments {
                                    irc.push(IR::Comment{comment: format!("Global {} via H[t252+{}]", name, stack_offset)});
                                }
                                // irc.push(IR::Assign2Op{target: IRVar::Var{varid: v}, val1: IRVar::Var{varid: 252}, op: IROp::Add, val2: IRVar::Number{value: o_i.unwrap()} });
                                st.symbols.insert(name.clone(), IRVar::Ht{stack_offset: stack_offset, local: false, orig_name: name.clone() });
                            }
                        },
                        Some(&IRVar::VH(ref boxed)) => {  // appears to be local for global level, so change local to false
                            match **boxed {
                                IRVar::Ht{stack_offset, local: true, orig_name: _} => {
                                    if optioncfg.fixed_global_access {
                                        let addr = st.bsp + stack_offset;  // calculate access index in H of globals each time
                                        if optioncfg.comments {
                                            irc.push(IR::Comment{comment: format!("Global {} via H[{}]", name, addr)});
                                        }
                                        st.symbols.insert(name.clone(), IRVar::VH(Box::new(IRVar::Hx{addr: addr})));
                                    } else {
                                        if optioncfg.comments {
                                            irc.push(IR::Comment{comment: format!("Global {} via H[t252+{}]", name, stack_offset)});
                                        }
                                        st.symbols.insert(name.clone(), IRVar::VH(Box::new(IRVar::Ht{stack_offset: stack_offset, local: false, orig_name: name.clone() })));
                                    }
                                },
                                IRVar::Hx{addr} => {  // not used
                                    if optioncfg.comments {
                                        irc.push(IR::Comment{comment: format!("Global {} via H[{}]", name, addr)});
                                    }
                                    st.symbols.insert(name.clone(), IRVar::VH(Box::new(IRVar::Hx{addr: addr})));
                                },
                                _ => match name.as_str() {
                                    "hh" | "hm" | "ph" | "pm" | "n" => { error!("can not use {} as global variable as it's read-only", name); panic!("error") },
                                    _ => { error!("can not find global variable {}", name); panic!("error") }
                                },
                            }
                        },
                        Some(&IRVar::VM(ref boxed)) => {
                            match **boxed {
                                IRVar::Ht{stack_offset, local: true, orig_name: _} => {
                                    if optioncfg.fixed_global_access {
                                        let addr = st.bsp + stack_offset;  // calculate access index in H of globals each time
                                        if optioncfg.comments {
                                            irc.push(IR::Comment{comment: format!("Global {} via H[{}]", name, addr)});
                                        }
                                        st.symbols.insert(name.clone(), IRVar::VM(Box::new(IRVar::Hx{addr: addr})));
                                    } else {
                                        if optioncfg.comments {
                                            irc.push(IR::Comment{comment: format!("Global {} via H[t252+{}]", name, stack_offset)});
                                        }
                                        st.symbols.insert(name.clone(), IRVar::VM(Box::new(IRVar::Ht{stack_offset: stack_offset, local: false, orig_name: name.clone() })));
                                    }
                                },
                                IRVar::Hx{addr} => { // not used
                                    if optioncfg.comments {
                                        irc.push(IR::Comment{comment: format!("Global {} via H[{}]", name, addr)});
                                    }
                                    st.symbols.insert(name.clone(), IRVar::VM(Box::new(IRVar::Hx{addr: addr})));
                                },
                                _ => match name.as_str() {
                                    "hh" | "hm" | "ph" | "pm" | "n" => { error!("can not use {} as global variable as it's read-only", name); panic!("error") },
                                    _ => { error!("can not find global variable {}", name); panic!("error") }
                                },
                            }
                        },
                        _ => match name.as_str() {
                                    "hh" | "hm" | "ph" | "pm" | "n" => { error!("can not use {} as global variable as it's read-only", name); panic!("error") },
                                    _ => { error!("can not find global variable {}", name); panic!("error") }
                             }
                    }
                }
            },
            &Stmt::Nonlocal{ref names, ref location} => {
                error!("{}: nonlocal {}\nnonlocal and closures are not supported, use scope of either local or global but not nested access", location, names[..].join(", ")); panic!("error");
            },
            &Stmt::Return{ref value, location: _} => {
                match value {
                    &Some(ref expr) => {
                        let (eval_ret, ret_var) = evaluate(expr, &mut st, optioncfg);
                        irc.extend_from_slice(&eval_ret[..]);
                        st.try_freeing_varid(&ret_var);
                        irc.push(IR::Return{var: Some(ret_var)}); // to be expanded in .convert() as it is a meta IR instruction
                    },
                    &None => { irc.push(IR::Return{var: None}); },
                }
            },
            &Stmt::If{ref test, ref body, ref orelse, location: _} => {
                let (eval_cond, cond) = evaluate(test, &mut st, optioncfg);
                irc.extend_from_slice(&eval_cond[..]);
                let else_label = st.get_new_label("else");
                let endif_label = st.get_new_label("endif");
                st.try_freeing_varid(&cond);
                irc.push(IR::IfN{cond_var: cond, goto_label: else_label.clone()});  // @TODO: look into test and use IfNeq/Eq is some caes? or via a post_ir pass
                irc.push(IR::Block{stmts: traverse(body, st, optioncfg)});
                irc.push(IR::GoTo{label: endif_label.clone()});
                irc.push(IR::Label{label: else_label});
                irc.push(IR::Block{stmts: traverse(orelse, st, optioncfg)});
                irc.push(IR::Label{label: endif_label});
            },
            &Stmt::While{ref test, ref body, ref orelse, location: _} => {
                let while_label = st.get_new_label("while");
                let whileelse_label = st.get_new_label("whileelse");
                let whileend_label = st.get_new_label("whileend");
                st.while_begins.push(while_label.clone());
                st.while_ends.push(whileend_label.clone());
                irc.push(IR::Label{label: while_label.clone()});
                let (eval_cond, cond) = evaluate(test, &mut st, optioncfg);
                irc.extend_from_slice(&eval_cond[..]);
                st.try_freeing_varid(&cond);
                irc.push(IR::IfN{cond_var: cond, goto_label: if !orelse.is_empty() { whileelse_label.clone() } else { whileend_label.clone() } });
                irc.push(IR::Block{stmts: traverse(body, st, optioncfg)});
                irc.push(IR::GoTo{label: while_label});
                if !orelse.is_empty() {
                    irc.push(IR::Label{label: whileelse_label.clone()});
                    irc.push(IR::Block{stmts: traverse(orelse, st, optioncfg)});
                }
                irc.push(IR::Label{label: whileend_label});
                st.while_begins.pop();
                st.while_ends.pop();
            },
            &Stmt::Break{location: _} => {
                let end = (*(st.while_ends.last().unwrap_or_else(|| panic!("break is not in while-loop")))).clone();
                irc.push(IR::GoTo{label: end});
            },
            &Stmt::Continue{location: _} => {
                let beginning = (*(st.while_begins.last().unwrap_or_else(|| panic!("continue is not in while-loop")))).clone();
                irc.push(IR::GoTo{label: beginning});
            },
        }
        temp_pair_vh = new_temp_pair_vh;  // used for optimisations
        temp_pair_vm = new_temp_pair_vm;
    }
    irc
}

pub fn gen_code(is_hcomp: bool, code: &[Stmt], zpaqcfgfile: &ZPAQCfgFile, source: String, optioncfg: &options::Options) -> Vec<IR> {
    let mut st = SymbolTable::new_from_model(zpaqcfgfile);
    st.source = source.lines().map(|l| l.to_string()).collect::<Vec<String>>();
    st.bsp = 2u32.pow(if is_hcomp { st.hh as u32 } else { st.ph as u32 }); // stack beginns after original size of H
    let mut irc = vec![
        IR::InitialCode{bsp: st.bsp},
    ];
    irc.extend_from_slice(&traverse(code, &mut st, optioncfg)[..]);
    if irc.len() == 1 { // return empty section if it only contains inital code
        return vec![];
    }
    irc.push(IR::Label{label: "call_next".to_string()}); // after h/pcomp execution finished, call it again with new input byte
    irc.push(IR::Assign{target: IRVar::Var{varid: 253}, source: IRVar::Number{value: 4294967294} });  // input_c = NONE-1
    irc.push(IR::Call{label: if is_hcomp { "hcomp".to_string() } else { "pcomp".to_string() }, args: vec![IRVar::Var{varid: 255}], stack_pos: st.stack_pos, ret_id: st.make_new_return_id()});
    // generate jump code for return positions
    irc.push(IR::JumpCode{ret_ids: (0..st.make_new_return_id() ).collect::<Vec<u32>>(), stackend: st.bsp + zpaqcfgfile.stacksize});
    irc
}

pub struct SymbolTable {
    // @TODO: ? table for functions→label (needs get_new_label in &FunctionDef), to allow local functions overwriting global functions
    pub symbols: HashMap<String, IRVar>,
    previous: Vec<HashMap<String, IRVar>>,
    pub live_ids: Vec<u8>,  // next free id
    previous_ids: Vec<Vec<u8>>,
    pub stack_pos: u32, // points to current used stack position as offset from bsp
    previous_stack_pos: Vec<u32>,
    pub while_begins: Vec<String>,
    pub while_ends: Vec<String>,

    pub hh: u8,
    pub hm: u8,
    pub ph: u8,
    pub pm: u8,
    pub n: u8,

    pub bsp: u32,
    label_id: u32,
    return_id: u32,

    last_line_printed: usize,
    pub source: Vec<String>,
}


impl SymbolTable {
    /// unique label
    pub fn get_new_label(&mut self, name: &str) -> String {
        self.label_id += 1;
        format!("{}_{}", name, self.label_id)
    }
    pub fn make_new_return_id(&mut self) -> u32 {
        let i = self.return_id;
        self.return_id += 1;
        i
    }
    pub fn make_stack_var(&mut self) -> u32 {
        self.stack_pos += 1;
        self.stack_pos
    }
    pub fn make_temp_var(&mut self) -> u8 {
        for v in 1..252 {  // one of 1 up to 251
            // t255 is reserved because it's used by InitialCode as inital value of the A register
            // and t254 is used as reading flag, t253 holds input_c, t252 holds the globalbsp
            if !self.live_ids.contains(&v) {
                self.live_ids.push(v);
                return v;
            }
        }
        error!("not enough temporary variables");
        panic!("error")
    }
    fn try_removing_varid(&mut self, varid: u8){
        if self.live_ids.contains(&varid) {
            for i in 0..self.live_ids.len() {
                if self.live_ids[i] == varid {  // it's assumed that it only exists once
                    self.live_ids.remove(i);
                    return;
                }
            }
        }
    }
    pub fn try_freeing_varid(&mut self, irvar: &IRVar) {
        match irvar.tovar() {
            IRVar::Var{varid} => { self.try_removing_varid(varid); },
            IRVar::M{index_varid} => { self.try_removing_varid(index_varid); },
            IRVar::H{index_varid, orig_name: _} => { self.try_removing_varid(index_varid); },
            _ => {},
        }
    }
    pub fn get_value(&mut self, id: &str, optioncfg: &options::Options) -> (IRVar, Vec<IR>) {
        let irc = vec![]; // mut
        // access to nested vars is not supported as closures are not, but global space is ok
        // let mut next_var = Some(self.make_temp_var());  // reserve free variable
        let r = match self.symbols.get(id) {
            Some(x) => x.clone(),
            None => if !self.previous.is_empty() { match self.previous[0].get(id) {
                        Some(&IRVar::Ht{stack_offset, local: true, orig_name: _}) => {  // appears local for global level
                            // let v = next_var.unwrap();
                            // next_var = None;
                            // irc.push(IR::Comment{comment: format!("access global {} via t{}", id, v)});
                            // irc.push(IR::Assign{target: IRVar::Var{varid: v}, source: IRVar::Number{value: self.bsp + o_i.unwrap()}});
                            // IRVar::H{index_varid: Some(v), orig_name: id.to_string(), stack_offset: None}
                            if optioncfg.fixed_global_access {
                                IRVar::Hx{addr: self.bsp + stack_offset}
                            } else {
                                IRVar::Ht{stack_offset: stack_offset, local: false, orig_name: id.to_string()}
                            }
                        },
                        Some(&IRVar::VH(ref boxed)) => {  // appears local for global level
                            match **boxed {
                                IRVar::Ht{stack_offset, local: true, orig_name: _} => {
                                    IRVar::VH(Box::new(if optioncfg.fixed_global_access {
                                        IRVar::Hx{addr: self.bsp + stack_offset}
                                    } else {
                                        IRVar::Ht{stack_offset: stack_offset, local: false, orig_name: id.to_string()}
                                    }))
                                },
                                ref x => { IRVar::VH(Box::new(x.clone())) },
                            }
                        },
                        Some(&IRVar::VM(ref boxed)) => {  // appears local for global level
                            match **boxed {
                                IRVar::Ht{stack_offset, local: true, orig_name: _} => {
                                    IRVar::VM(Box::new(if optioncfg.fixed_global_access {
                                        IRVar::Hx{addr: self.bsp + stack_offset}
                                    } else {
                                        IRVar::Ht{stack_offset: stack_offset, local: false, orig_name: id.to_string()}
                                    }))
                                },
                                ref x => { IRVar::VM(Box::new(x.clone())) },
                            }
                        },
                        Some(x) => x.clone(),
                        None => match id {
                                    "hh" => IRVar::Number{value: self.hh as u32},
                                    "hm" => IRVar::Number{value: self.hm as u32},
                                    "ph" => IRVar::Number{value: self.ph as u32},
                                    "pm" => IRVar::Number{value: self.pm as u32},
                                    "n" => IRVar::Number{value: self.n as u32},
                                    "NONE" => IRVar::Number{value: 4294967295},
                                    _ => { error!("variable {} not found", id); panic!("error") }
                                }
                    } } else {
                        match id {
                                    "hh" => IRVar::Number{value: self.hh as u32},
                                    "hm" => IRVar::Number{value: self.hm as u32},
                                    "ph" => IRVar::Number{value: self.ph as u32},
                                    "pm" => IRVar::Number{value: self.pm as u32},
                                    "n" => IRVar::Number{value: self.n as u32},
                                    "NONE" => IRVar::Number{value: 4294967295},
                                    _ => { error!("variable {} not found", id); panic!("error") }
                                }
                    }
        };
        (r, irc)
    }
    pub fn new() -> SymbolTable {
        SymbolTable{symbols: HashMap::<String, IRVar>::new(), previous: vec![],
        live_ids: vec![], previous_ids: vec![], stack_pos: 0, previous_stack_pos: vec![], return_id: 0, bsp: 0, last_line_printed: 0, source: vec![],
        hh: 0, hm: 0, ph: 0, pm: 0, n: 0, label_id: 0, while_begins: vec![], while_ends: vec![]}
    }
    pub fn new_from_model(zpaqcfgfile: &ZPAQCfgFile) -> SymbolTable {
        let mut st = SymbolTable::new();
        st.hh = zpaqcfgfile.hh;
        st.hm = zpaqcfgfile.hm;
        st.ph = zpaqcfgfile.ph;
        st.pm = zpaqcfgfile.pm;
        st.n = zpaqcfgfile.n;
        st
    }
    pub fn push(&mut self) {  // enter new scope
        self.previous.push(self.symbols.clone());
        self.symbols = HashMap::<String, IRVar>::new();
        self.previous_ids.push(self.live_ids.clone());
        self.live_ids.clear();
        self.previous_stack_pos.push(self.stack_pos);
        self.stack_pos = 0;
    }
    pub fn pop(&mut self) {  // return to outer scope
        self.symbols = self.previous.pop().unwrap();
        self.live_ids = self.previous_ids.pop().unwrap();
        self.stack_pos = self.previous_stack_pos.pop().unwrap();
    }
    /// deliver corresponding source code line for location tag once, afterwards it will return None
    pub fn source_line(&mut self, location: String) -> Option<String> {
        let current_line = match usize::from_str(location.split(',').next().unwrap()) {
            Ok(v) => v,
            _ => { return None; },
        };
        if current_line != self.last_line_printed {
            self.last_line_printed = current_line;
            Some(format!("                    {}: {}", self.last_line_printed, self.source[self.last_line_printed-1]))
        } else {
            None
        }
    }
}

/// extracts values of context model configuration
pub fn read_context_model(parsed_stmts: &[Stmt], optioncfg: &options::Options) -> ZPAQCfgFile {
    // default stack size is 2^20 words, i.e. 1024 KiB = 1 MiB
    let mut zpaqcfgfile = ZPAQCfgFile{finalised: false, stacksize: optioncfg.stacksize, hh: 0, hm: 0, ph: 0, pm: 0, n: 0, model: vec![], hcomp: vec![], pcomp: vec![], pcomp_invocation: "".to_string()};
    for node in parsed_stmts {
        match node {
            &Stmt::Assign{ref target, ref value, location: _} => {
              match **target {
                Expr::Name{ref id, ctx: ExprContext::Store, location: _} => {
                  match &id[..] {
                      "hh" => match **value { Expr::Num{n, location: _} => { zpaqcfgfile.hh = n as u8; } , _ => {} },
                      "hm" => match **value { Expr::Num{n, location: _} => { zpaqcfgfile.hm = n as u8; } , _ => {} },
                      "ph" => match **value { Expr::Num{n, location: _} => { zpaqcfgfile.ph = n as u8; } , _ => {} },
                      "pm" => match **value { Expr::Num{n, location: _} => { zpaqcfgfile.pm = n as u8; } , _ => {} },
                      "pcomp_invocation" => match **value { Expr::Str{ref s, location: _} => {zpaqcfgfile.pcomp_invocation = s.clone() } , _ => {} },
                      "n" => match **value { Expr::Call{ref func, ref args, keywords: _, location: _} => {
                          if func == "len" {  // e.g. n = len({ 0: "cm 19 22 (comment)", 1: "const 160", })
                              match args[0] {
                                Expr::Dict{ref keys, ref values, location: _} => {
                                  zpaqcfgfile.n = keys.len() as u8;
                                  for k in 0..keys.len() {
                                    let i = match keys[k] { Expr::Num{n, location: _} => n , _=> 0};
                                    let c = match values[k] { Expr::Str{ref s, location: _} => s.clone() , _ => "".to_string() };
                                    zpaqcfgfile.model.push((i as u8, c));
                                  }
                                },
                                _ => {}
                              }
                          }
                          } , _ => {} },
                      _ => {}
                  }},
                _ => {}
              }
            },
            _ => {}
        }
    }
    zpaqcfgfile
}

