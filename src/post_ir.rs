use ir::{IR, IRVar};
use options;

pub fn optimise(ir_code: Vec<IR>, optioncfg: &options::Options) -> Vec<IR> {
    let deblocked_no_unused_functions = remove_unused_functions(deblock(ir_code));
    if !optioncfg.disable_optim {
        lighten_save_load(remove_unused_assignments(deblocked_no_unused_functions), optioncfg)
    } else {
        deblocked_no_unused_functions
    }
}

pub fn deblock(ir_code: Vec<IR>) -> Vec<IR> {
    let mut irc = vec![];
    for cmd in ir_code {
        match cmd {
            IR::Block{stmts} => { irc.extend_from_slice(&deblock(stmts)); },
            c => { irc.push(c); },
        }
    }
    irc
}

// maybe in the future: move code out of loop for speed?

pub fn remove_unused_assignments(ir_code: Vec<IR>) -> Vec<IR> {
    // @TODO:
    // keep original variable if it's read-only and not changed
    // replace if with ifNeq? in gen_ir?! +++ IfNeq/Eq in gen_zpaql.rs, parser, cases and match on eval
    // ir:if with ==? how to optimise? IR-pass: t3=a!=b,
    //       replace if t3 goto with ifNeq a b (before unusedassignments pass)
    // t8 = H[17] * 3
    // H[18] = t8

    let mut irc = vec![];
    for cmd in ir_code {
        match cmd {
            IR::Assign{target: IRVar::Var{varid: a}, source: IRVar::Var{varid: b}} => {
                if a != b {
                    irc.push(IR::Assign{target: IRVar::Var{varid: a}, source: IRVar::Var{varid: b}});
                }
            },
            c => { irc.push(c); },
        }
    }
    irc
}

pub fn remove_unused_functions(mut ir_code: Vec<IR>) -> Vec<IR> {
    loop {
        let mut used = vec![];
        let mut irc = vec![];
        for cmd in ir_code.iter() {
            match cmd {
                &IR::Call{ref label, args: _, stack_pos: _, ret_id: _} => { used.push(label.clone()); },
                _ => {},
            }
        }
        let mut deleted =  0;
        let mut in_func = "".to_string();  // end label of function to be removed
        let mut remove_ret_ids = vec![];
        for cmd in ir_code {
            match cmd {
                IR::GoTo{label} => {
                    if in_func.is_empty() && label.ends_with("_end~") {
                        if !used.contains(&(label[..label.len()-5].to_string())) {
                            debug!("removing function {}", &label[..label.len()-5]);
                            in_func = label;
                        } else {
                            irc.push(IR::GoTo{label: label});
                        }
                    } else if in_func.is_empty() {
                        irc.push(IR::GoTo{label: label});
                    }
                },
                IR::Label{label} => {
                    if in_func.is_empty() {
                        irc.push(IR::Label{label: label});
                    } else if label == in_func {
                        in_func = "".to_string();
                        deleted += 1;
                    }
                },
                IR::Call{label, args, stack_pos, ret_id} => {
                    if in_func.is_empty() {
                        irc.push(IR::Call{label: label, args: args, stack_pos: stack_pos, ret_id: ret_id});
                    } else {
                        remove_ret_ids.push(ret_id);
                    }
                },
                IR::JumpCode{mut ret_ids, stackend} => {
                    if in_func.is_empty() {
                        ret_ids.retain(|i| !remove_ret_ids.contains(i) );
                        irc.push(IR::JumpCode{ret_ids: ret_ids, stackend: stackend});
                    }
                },
                c => {
                    if in_func.is_empty() {
                        irc.push(c);
                    }
                },
            }
        }
        ir_code = irc;
        if deleted == 0 {
            break;
        }
    }
    ir_code
}


// @TODO: if tx holds a non-temporary variable, but e.g. a local variabe, it needs to be live
// until the end of a while loop! currently not a problem as all tx are done after a single python statement

/// Lifetime optimisation to exclude non-live temporary variables from being stored on stack before a call.
/// Expects to get input from deblock(), so a flat vec without blocks
pub fn lighten_save_load(mut ir_code: Vec<IR>, optioncfg: &options::Options) -> Vec<IR> {
    let mut live_ids = vec![];
    let mut non_store_live_ids = vec![];
    let mut in_scope = false;
    let mut left_in_scope = false;
    let mut irc = vec![];
    ir_code.reverse();
    for cmd in ir_code {
        let c = cmd.clone();
        match (cmd, in_scope) {
            (IR::MarkTempVarEnd, true) => {  // @TODO: support with push on vectors like in symboltable
                error!("inner function detected, not yet supported for optimisation, use --disable-optim");
                if !optioncfg.ignore_errors {
                    panic!("error");
                }
            },
            (IR::MarkTempVarEnd, false) => {
                in_scope = true;
                live_ids.clear();
                non_store_live_ids.clear();
                irc.push(c);
            },
            (IR::MarkTempVarStart, true) => {
                in_scope = false;
                live_ids.clear();
                non_store_live_ids.clear();
                irc.push(c);
            },
            (IR::Assign{target, source}, true) => {
                match target {
                    IRVar::Var{varid} => { live_ids.retain(|&x| x != varid); },
                    IRVar::M{index_varid} => { live_ids.push(index_varid); },
                    IRVar::H{index_varid, orig_name: _} => { live_ids.push(index_varid); },
                    _ => {},
                }
                match source {
                    IRVar::Var{varid} => { live_ids.push(varid); },
                    IRVar::M{index_varid} => { live_ids.push(index_varid); },
                    IRVar::H{index_varid, orig_name: _} => { live_ids.push(index_varid); },
                    _ => {},
                }
                irc.push(c);
            },
            (IR::Assign2Op{target, val1, op: _, val2}, true) => {
                match target {
                    IRVar::Var{varid} => { live_ids.retain(|&x| x != varid); },
                    IRVar::M{index_varid} => { live_ids.push(index_varid); },
                    IRVar::H{index_varid, orig_name: _} => { live_ids.push(index_varid); },
                    _ => {},
                }
                match val1 {
                    IRVar::Var{varid} => { live_ids.push(varid); },
                    IRVar::M{index_varid} => { live_ids.push(index_varid); },
                    IRVar::H{index_varid, orig_name: _} => { live_ids.push(index_varid); },
                    _ => {},
                }
                match val2 {
                    IRVar::Var{varid} => { live_ids.push(varid); },
                    IRVar::M{index_varid} => { live_ids.push(index_varid); },
                    IRVar::H{index_varid, orig_name: _, } => { live_ids.push(index_varid); },
                    _ => {},
                }
                irc.push(c);
            },
            (IR::Assign1Op{target, uop: _, source}, true) => {
                match target {
                    IRVar::Var{varid} => { live_ids.retain(|&x| x != varid); },
                    IRVar::M{index_varid} => { live_ids.push(index_varid); },
                    IRVar::H{index_varid, orig_name: _} => { live_ids.push(index_varid); },
                    _ => {},
                }
                match source {
                    IRVar::Var{varid} => { live_ids.push(varid); },
                    IRVar::M{index_varid} => { live_ids.push(index_varid); },
                    IRVar::H{index_varid, orig_name: _} => { live_ids.push(index_varid); },
                    _ => {},
                }
                irc.push(c);
            },
            (IR::Out{var}, true) => {
                match var {
                    IRVar::Var{varid} => { live_ids.push(varid); },
                    IRVar::M{index_varid} => { live_ids.push(index_varid); },
                    IRVar::H{index_varid, orig_name: _} => { live_ids.push(index_varid); },
                    _ => {},
                }
                irc.push(c);
            },
            (IR::If{cond_var, goto_label: _}, true) => {
                match cond_var {
                    IRVar::Var{varid} => { live_ids.push(varid); },
                    IRVar::M{index_varid} => { live_ids.push(index_varid); },
                    IRVar::H{index_varid, orig_name: _} => { live_ids.push(index_varid); },
                    _ => {},
                }
                irc.push(c);
            },
            (IR::IfN{cond_var, goto_label: _}, true) => {
                match cond_var {
                    IRVar::Var{varid} => { live_ids.push(varid); },
                    IRVar::M{index_varid} => { live_ids.push(index_varid); },
                    IRVar::H{index_varid, orig_name: _} => { live_ids.push(index_varid); },
                    _ => {},
                }
                irc.push(c);
            },
            (IR::IfEq{val1, val2, goto_label: _}, true) => {
                match val1 {
                    IRVar::Var{varid} => { live_ids.push(varid); },
                    IRVar::M{index_varid} => { live_ids.push(index_varid); },
                    IRVar::H{index_varid, orig_name: _} => { live_ids.push(index_varid); },
                    _ => {},
                }
                match val2 {
                    IRVar::Var{varid} => { live_ids.push(varid); },
                    IRVar::M{index_varid} => { live_ids.push(index_varid); },
                    IRVar::H{index_varid, orig_name: _} => { live_ids.push(index_varid); },
                    _ => {},
                }
                irc.push(c);
            },
            (IR::IfNeq{val1, val2, goto_label: _}, true) => {
                match val1 {
                    IRVar::Var{varid} => { live_ids.push(varid); },
                    IRVar::M{index_varid} => { live_ids.push(index_varid); },
                    IRVar::H{index_varid, orig_name: _} => { live_ids.push(index_varid); },
                    _ => {},
                }
                match val2 {
                    IRVar::Var{varid} => { live_ids.push(varid); },
                    IRVar::M{index_varid} => { live_ids.push(index_varid); },
                    IRVar::H{index_varid, orig_name: _} => { live_ids.push(index_varid); },
                    _ => {},
                }
                irc.push(c);
            },
            (IR::Return{var: Some(varx)}, true) => {
                match varx {
                    IRVar::Var{varid} => { live_ids.push(varid); },
                    IRVar::M{index_varid} => { live_ids.push(index_varid); },
                    IRVar::H{index_varid, orig_name: _} => { live_ids.push(index_varid); },
                    _ => {},
                }
                irc.push(c);
            },
            (IR::Call{label: _, args, stack_pos: _, ret_id: _}, true) => {
                for var in args {
                    match var {
                        IRVar::Var{varid} => { live_ids.push(varid); },
                        IRVar::M{index_varid} => { live_ids.push(index_varid); },
                        IRVar::H{index_varid, orig_name: _} => { live_ids.push(index_varid); },
                        _ => {},
                    }
                }
                irc.push(c);
            },
            (IR::Call{label: _, args, stack_pos: _, ret_id: _}, false) => {  // sometimes arguments live time spans over mutiple store load cyles if the call arguments are calles themselves
                for var in args {
                    match var {
                        IRVar::Var{varid} => { non_store_live_ids.push(varid); },
                        IRVar::M{index_varid} => { non_store_live_ids.push(index_varid); },
                        IRVar::H{index_varid, orig_name: _} => { non_store_live_ids.push(index_varid); },
                        _ => {},
                    }
                }
                irc.push(c);
            },
            (IR::StoreTempVars{ti, stack_pos}, false) => {
                // in_scope is false here so that return assignment and call arguments are not counted as live,
                // but exactly the same list as in LoadTempVars is used
                if left_in_scope {
                    let mut tn = vec![];
                    debug!("Store as before {:?}", ti);
                    for i in ti {
                        if live_ids.contains(&i) {
                            tn.push(i);
                        }
                    }
                    debug!("Store after optimisation {:?}", tn);
                    irc.push(IR::StoreTempVars{ti: tn, stack_pos: stack_pos});
                    in_scope = true;
                    left_in_scope = false;
                    for i in non_store_live_ids.iter() {  // they are needed as call arguments after store but don't need to be stored
                        if !live_ids.contains(i) {
                            live_ids.push(*i);
                        }
                    }
                    non_store_live_ids.clear();
                } else {
                    irc.push(c);
                }
            },
            (IR::LoadTempVars{ti, stack_pos}, true) => {
                let mut tn = vec![];
                debug!("Load as before {:?}", ti);
                for i in ti {
                    if live_ids.contains(&i) {
                        tn.push(i);
                    }
                }
                debug!("Load after optimisation {:?}", tn);
                irc.push(IR::LoadTempVars{ti: tn, stack_pos: stack_pos});
                in_scope = false;
                left_in_scope = true;
                non_store_live_ids.clear();
            },
            (_, _) => { irc.push(c); },
        }
    }
    irc.reverse();
    irc
}

