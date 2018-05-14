use zpaql::{ZPAQLOp, Loc, Reg, OtherReg, SwapLoc};
use ir::{IR, IRVar, IROp, IRUOp};
use options;

use std::collections::HashMap;

/// keeps track of variable copies in registers or memory locations
pub struct Cache {
    pub last_hold: HashMap<Loc, IRVar>,
}

impl Cache {
    /// a modification of a register also needs to remove the cache entry for the location in points to in memory
    pub fn remove_reg(&mut self, loc: &Loc) {
        match loc {
            &Loc::Reg(Reg::OtherReg(OtherReg::D)) => {
                self.last_hold.remove(&Loc::Reg(Reg::OtherReg(OtherReg::D)));
                self.last_hold.remove(&Loc::HD);
            },
            &Loc::Reg(Reg::OtherReg(OtherReg::C)) => {
                self.last_hold.remove(&Loc::Reg(Reg::OtherReg(OtherReg::C)));
                self.last_hold.remove(&Loc::MC);
            },
            &Loc::Reg(Reg::OtherReg(OtherReg::B)) => {
                self.last_hold.remove(&Loc::Reg(Reg::OtherReg(OtherReg::B)));
                self.last_hold.remove(&Loc::MB);
            },
            x => { self.last_hold.remove(x); },
        }
    }
    /// invalidate all cache entries for R-variable copies because of a new value
    pub fn delete_references(&mut self, varid: u8) {
        let mut new = HashMap::<Loc, IRVar>::new();
        for (k, v) in self.last_hold.iter() {
            match v {
                &IRVar::H{index_varid, orig_name: _} if index_varid == varid => {},
                &IRVar::M{index_varid} if index_varid == varid => {},
                &IRVar::Ht{stack_offset: _, local, orig_name: _} if (local && varid == 0) || (!local && varid == 252) => {},
                _ => { new.insert(k.clone(), v.clone()); },
            }
        }
        self.last_hold = new;
    }
    /// delete cache entries for this variable
    pub fn delete(&mut self, irvar: &IRVar) {
        let mut new = HashMap::<Loc, IRVar>::new();
        for (k, v) in self.last_hold.iter() {
            if v != &(irvar.tovar()) {
                new.insert(k.clone(), v.clone());
            }
        }
        self.last_hold = new;
    }
    /// delete cache entries for this variable but skip the mentioned location
    pub fn delete_not(&mut self, irvar: &IRVar, loc: &Loc) {
        let mut new = HashMap::<Loc, IRVar>::new();
        for (k, v) in self.last_hold.iter() {
            if v != &(irvar.tovar()) || k == loc {
                new.insert(k.clone(), v.clone());
            }
        }
        self.last_hold = new;
    }
    /// test if the location holds a copy of the variable
    pub fn is_loc(&self, loc: &Loc, irvar: &IRVar) -> bool {
        match self.last_hold.get(loc) {
            Some(&IRVar::Ht{stack_offset: so, local: lo, ref orig_name}) => {
                match irvar {
                    &IRVar::Ht{stack_offset, local, orig_name: _} => stack_offset == so && local == lo, // needed to ignore orig_name
                    vv => vv == &IRVar::Ht{stack_offset: so, local: lo, orig_name: orig_name.clone()},
                }
            },
            Some(&IRVar::H{index_varid: iv, ref orig_name}) => {
                match irvar {
                    &IRVar::H{index_varid, orig_name: _} => index_varid == iv,  // needed to ignore orig_name
                    vv => vv == &IRVar::H{index_varid: iv, orig_name: orig_name.clone()},
                }
            },
            Some(v) => irvar == v,
            _ => false,
        }
    }
}

/// compile IR code (which works on H, M and R) to ZPAQL code by using the registers A-D
pub fn emit_zpaql(irc: &[IR], ch: &mut Cache, optioncfg: &options::Options) -> Vec<ZPAQLOp> {
    let mut code = vec![];
    for op in irc {
        match op.convert() {  // write original IR statement as comment
            IR::Block{stmts: _} => {},
            IR::Comment{comment: _} => {},
            other => { if optioncfg.comments { code.push(ZPAQLOp::Comment{comment: format!("        {}", other)}); } },
        }
        match op.convert() {
            IR::Label{ref label} => {
                ch.last_hold.clear(); // label is jump destination, can't use any cache from before
                code.push(ZPAQLOp::Label{label: label.clone(), position: 0});
            }, // position will be set afterwards
            IR::GoTo{ref label} => { code.push(ZPAQLOp::GoTo{label: label.clone()}); },
            IR::Error => { code.push(ZPAQLOp::Error); },
            IR::Halt => { code.push(ZPAQLOp::Halt); },
            IR::Comment{ref comment} => {
                code.push(ZPAQLOp::Comment{comment: comment.clone()});
            },
            IR::Out{ref var} => {
                code.extend_from_slice(&assign_var_to_loc(var, &Loc::Reg(Reg::A), ch));
                code.push(ZPAQLOp::Out);
            },
            IR::If{ref cond_var, ref goto_label} => {
                code.extend_from_slice(&assign_var_to_loc(cond_var, &Loc::Reg(Reg::A), ch));
                code.push(ZPAQLOp::AeqN{n: 0});
                code.push(ZPAQLOp::JT{n: 3});  // cond is false, so jump over the jump, i.e. incr. PC by 3 more than normal given that GoTo will be a LJ
                code.push(ZPAQLOp::GoTo{label: goto_label.clone()});
            },
            IR::IfN{ref cond_var, ref goto_label} => {
                code.extend_from_slice(&assign_var_to_loc(cond_var, &Loc::Reg(Reg::A), ch));
                code.push(ZPAQLOp::AeqN{n: 0});
                code.push(ZPAQLOp::JF{n: 3});  // cond is true, so jump over the jump, i.e. incr. PC by 3 more than normal given that GoTo will be a LJ
                code.push(ZPAQLOp::GoTo{label: goto_label.clone()});
            },
            IR::IfEq{ref val1, ref val2, ref goto_label} => {
                code.extend_from_slice(&assign_var_to_loc(val1, &Loc::Reg(Reg::OtherReg(OtherReg::C)), ch));
                code.extend_from_slice(&assign_var_to_loc(val2, &Loc::Reg(Reg::A), ch));
                code.push(ZPAQLOp::Aeq(Loc::Reg(Reg::OtherReg(OtherReg::C))));
                code.push(ZPAQLOp::JF{n: 3});  // cond is true, so jump over the jump, i.e. incr. PC by 3 more than normal given that GoTo will be a LJ
                code.push(ZPAQLOp::GoTo{label: goto_label.clone()});
            },
            IR::IfNeq{ref val1, ref val2, ref goto_label} => {
                code.extend_from_slice(&assign_var_to_loc(val1, &Loc::Reg(Reg::OtherReg(OtherReg::C)), ch));
                code.extend_from_slice(&assign_var_to_loc(val2, &Loc::Reg(Reg::A), ch));
                code.push(ZPAQLOp::Aeq(Loc::Reg(Reg::OtherReg(OtherReg::C))));
                code.push(ZPAQLOp::JT{n: 3});  // cond is true, so jump over the jump, i.e. incr. PC by 3 more than normal given that GoTo will be a LJ
                code.push(ZPAQLOp::GoTo{label: goto_label.clone()});
            },
            IR::Block{ref stmts} => { code.extend_from_slice(&emit_zpaql(stmts, ch, optioncfg)) },  // recursively
            IR::Assign{ref target, ref source} => {
                if target != source {
                    match target.tovar() {
                        IRVar::Var{varid: _} => {  // assignments to R can only come from A
                            code.extend_from_slice(&assign_var_to_loc(source, &Loc::Reg(Reg::A), ch));
                            code.extend_from_slice(&assign_loc_to_var(target, &Loc::Reg(Reg::A), ch ));
                        },
                        IRVar::H{index_varid: _, orig_name: _} | IRVar::Ht{stack_offset: _, local: _, orig_name: _} | IRVar::Hx{addr: _} => {
                            match source.tovar() {
                                IRVar::Number{value: _} | IRVar::Var{varid: _} | IRVar::M{index_varid: _} | IRVar::Mx{addr: _} => {
                                    let (zc, loc) = gen_loc_for_var(target, ch);  // first make target ready, so it can be efficiently zeroed or increased
                                    code.extend_from_slice(&zc);
                                    ch.delete(target);  // because target will get a new value
                                    code.extend_from_slice(&assign_var_to_loc(source, &loc, ch));  // copy variable to target location
                                    ch.last_hold.insert(loc.clone(), target.clone()); // performs better then keeping loc->source mapping
                                    // otherwise one could also try something like:
                                    // let (zc, loc) = gen_loc_for_var(source);
                                    // code.extend_from_slice(&zc);
                                    // code.extend_from_slice(&assign_loc_to_var(target, &loc));
                                },
                                IRVar::Hx{addr: _} | IRVar::Ht{stack_offset: _, local: _, orig_name: _} | IRVar::H{index_varid: _, orig_name: _} => {
                                    // use C to hold the value because A could be needed during the calculation
                                    code.extend_from_slice(&assign_var_to_loc(source, &Loc::Reg(Reg::OtherReg(OtherReg::C)), ch));
                                    code.extend_from_slice(&assign_loc_to_var(target, &Loc::Reg(Reg::OtherReg(OtherReg::C)), ch));
                                },
                                _ => unreachable!(),
                            }
                        },
                        IRVar::M{index_varid: _} | IRVar::Mx{addr: _}=> {
                            let (zc, loc) = gen_loc_for_var(target, ch);  // improvement over var->loc->var
                            code.extend_from_slice(&zc);
                            ch.delete(target);
                            code.extend_from_slice(&assign_var_to_loc(source, &loc, ch));
                            ch.last_hold.insert(loc.clone(), target.clone());  // not measured yet if it makes a big difference or could be omitted
                        },
                        x => { error!("can't assign to {}", x); panic!("error") },
                    }
                }
            },
            IR::Assign1Op{ref target, uop, ref source} => {
                if target == source {
                    match target.tovar() {
                        IRVar::Var{varid: _} => {  // can not be increased in place, needs A
                            code.extend_from_slice(&assign_var_to_loc(source, &Loc::Reg(Reg::A), ch));
                            match uop {
                                IRUOp::Not => {  // (== 0)
                                    code.push(ZPAQLOp::Inc(Loc::Reg(Reg::A)));
                                    code.push(ZPAQLOp::AeqN{n: 1});
                                    code.push(ZPAQLOp::JT{n: 1});
                                    code.push(ZPAQLOp::Zero(Loc::Reg(Reg::A)));
                                },
                                IRUOp::Invert => { code.push(ZPAQLOp::Not(Loc::Reg(Reg::A)) ); },  // bitwise ~
                                IRUOp::USub => { code.push(ZPAQLOp::Not(Loc::Reg(Reg::A)) ); code.push(ZPAQLOp::Inc(Loc::Reg(Reg::A))); }, // -x == ~x + 1
                            }
                            ch.remove_reg(&Loc::Reg(Reg::A));
                            code.extend_from_slice(&assign_loc_to_var(target, &Loc::Reg(Reg::A), ch));
                        },
                        _ => { // can be modified in place
                            let (zc, loc) = gen_loc_for_var(target, ch);
                            ch.delete_not(target, &loc);
                            code.extend_from_slice(&zc);
                            match uop {
                                IRUOp::Not => {  // (== 0)
                                    code.push(ZPAQLOp::Zero(Loc::Reg(Reg::A)));
                                    ch.last_hold.insert(Loc::Reg(Reg::A), IRVar::Number{value: 0});
                                    code.push(ZPAQLOp::Aeq(loc.clone()) );
                                    code.push(ZPAQLOp::Zero(loc.clone()));
                                    code.push(ZPAQLOp::JF{n: 1});
                                    code.push(ZPAQLOp::Inc(loc.clone()));
                                },
                                IRUOp::Invert => { code.push(ZPAQLOp::Not(loc.clone() )); },  // bitwise ~
                                IRUOp::USub => { code.push(ZPAQLOp::Not(loc.clone() )); code.push(ZPAQLOp::Inc(loc.clone()) ); }, // -x == ~x + 1
                            }
                            ch.remove_reg(&loc);
                            ch.last_hold.insert(loc.clone(), target.clone());
                        },
                    }
                } else { // first copied to A, then calculated and then assigned to target
                    code.extend_from_slice(&assign_var_to_loc(source, &Loc::Reg(Reg::A), ch));
                    match uop {
                        IRUOp::Not => {  // (== 0)
                            code.push(ZPAQLOp::Inc(Loc::Reg(Reg::A)));
                            code.push(ZPAQLOp::AeqN{n: 1});
                            code.push(ZPAQLOp::JT{n: 1});
                            code.push(ZPAQLOp::Zero(Loc::Reg(Reg::A)));
                        },
                        IRUOp::Invert => { code.push(ZPAQLOp::Not(Loc::Reg(Reg::A)) ); },  // bitwise ~
                        IRUOp::USub => { code.push(ZPAQLOp::Not(Loc::Reg(Reg::A)) ); code.push(ZPAQLOp::Inc(Loc::Reg(Reg::A))); }, // -x == ~x + 1
                    }
                    ch.remove_reg(&Loc::Reg(Reg::A));
                    code.extend_from_slice(&assign_loc_to_var(target, &Loc::Reg(Reg::A), ch));
                }
            },
            IR::Assign2Op{ref target, ref val1, op, ref val2} => {
                if target == val1 && (op == IROp::Add || op == IROp::Sub) && (val2 == &IRVar::Number{value: 0} || val2 == &IRVar::Number{value: 1}) {
                    match val2 { // val1 = val1 + 1 (or + 0)
                        &IRVar::Number{value: 0} => {}, // nothing to do
                        _ => {
                            match target.tovar() {
                                IRVar::Var{varid: _} => {  // assignments to R must go though A
                                    code.extend_from_slice(&assign_var_to_loc(val1, &Loc::Reg(Reg::A), ch));
                                    if op == IROp::Add {
                                        code.push(ZPAQLOp::Inc(Loc::Reg(Reg::A)));
                                    } else { code.push(ZPAQLOp::Dec(Loc::Reg(Reg::A))); }
                                    ch.remove_reg(&Loc::Reg(Reg::A));
                                    code.extend_from_slice(&assign_loc_to_var(target, &Loc::Reg(Reg::A), ch));
                                },
                                _ => {  // other locations of the target variable can be inc/decreased directly
                                    let (zc, loc) = gen_loc_for_var(target, ch);
                                    code.extend_from_slice(&zc);
                                    ch.delete_not(target, &loc);
                                    if op == IROp::Add {
                                        code.push(ZPAQLOp::Inc(loc.clone()));
                                    } else { code.push(ZPAQLOp::Dec(loc.clone())); }
                                    ch.remove_reg(&loc);
                                    ch.last_hold.insert(loc.clone(), target.clone());
                                },
                            }
                        },
                    }
                } else {
                    // save val2 in C and val1 in A
                    code.extend_from_slice(&assign_var_to_loc(val2, &Loc::Reg(Reg::OtherReg(OtherReg::C)), ch));
                    code.extend_from_slice(&assign_var_to_loc(val1, &Loc::Reg(Reg::A), ch));
                    // calculate A = A <op> C
                    match op {
                        IROp::Add => { code.push(ZPAQLOp::Aadd(Loc::Reg(Reg::OtherReg(OtherReg::C)))); },
                        IROp::Sub => { code.push(ZPAQLOp::Asub(Loc::Reg(Reg::OtherReg(OtherReg::C)))); },
                        IROp::Mult => { code.push(ZPAQLOp::Amult(Loc::Reg(Reg::OtherReg(OtherReg::C)))); },
                        IROp::Div => { code.push(ZPAQLOp::Adiv(Loc::Reg(Reg::OtherReg(OtherReg::C)))); },
                        IROp::Pow => {
                            code.push(ZPAQLOp::Set{target: Loc::Reg(Reg::OtherReg(OtherReg::B)), source: Loc::Reg(Reg::A)});
                            code.push(ZPAQLOp::SetN{target: Loc::Reg(Reg::A), n: 1});
                            // loop start
                            code.push(ZPAQLOp::SwapA(SwapLoc::OtherReg(OtherReg::C)));
                            code.push(ZPAQLOp::AgtN{n: 0});
                            code.push(ZPAQLOp::JF{n: 5});  // PCnextInstr += ((N+128) mod 256) - 128
                            code.push(ZPAQLOp::SwapA(SwapLoc::OtherReg(OtherReg::C)));
                            code.push(ZPAQLOp::Amult(Loc::Reg(Reg::OtherReg(OtherReg::B))));
                            code.push(ZPAQLOp::Dec(Loc::Reg(Reg::OtherReg(OtherReg::C))));
                            code.push(ZPAQLOp::JMP{n: -10});
                            // jump finish
                            code.push(ZPAQLOp::SwapA(SwapLoc::OtherReg(OtherReg::C)));
                        },
                        IROp::LShift => { code.push(ZPAQLOp::Alshift(Loc::Reg(Reg::OtherReg(OtherReg::C))) ); },
                        IROp::RShift => { code.push(ZPAQLOp::Arshift(Loc::Reg(Reg::OtherReg(OtherReg::C))) ); },
                        IROp::Mod => { code.push(ZPAQLOp::Amod(Loc::Reg(Reg::OtherReg(OtherReg::C))) ); },

                        IROp::BitOr => { code.push(ZPAQLOp::Aor(Loc::Reg(Reg::OtherReg(OtherReg::C))) ); },
                        IROp::BitXor => { code.push(ZPAQLOp::Axor(Loc::Reg(Reg::OtherReg(OtherReg::C))) ); },
                        IROp::BitAnd => { code.push(ZPAQLOp::Aand(Loc::Reg(Reg::OtherReg(OtherReg::C))) ); },

                        IROp::Or => {
                            code.push(ZPAQLOp::AeqN{n: 0});
                            code.push(ZPAQLOp::JF{n: 1});
                            code.push(ZPAQLOp::Set{target: Loc::Reg(Reg::A), source: Loc::Reg(Reg::OtherReg(OtherReg::C))} );
                        },
                        IROp::And => {
                            code.push(ZPAQLOp::AeqN{n: 0});
                            code.push(ZPAQLOp::JT{n: 1});
                            code.push(ZPAQLOp::Set{target: Loc::Reg(Reg::A), source: Loc::Reg(Reg::OtherReg(OtherReg::C))} );
                        },
                        IROp::Eq => {
                            code.push(ZPAQLOp::Aeq(Loc::Reg(Reg::OtherReg(OtherReg::C))) );
                            code.push(ZPAQLOp::Zero(Loc::Reg(Reg::A)) );
                            code.push(ZPAQLOp::JF{n: 1});
                            code.push(ZPAQLOp::Inc(Loc::Reg(Reg::A)) );
                        },
                        IROp::NotEq => {
                            code.push(ZPAQLOp::Aeq(Loc::Reg(Reg::OtherReg(OtherReg::C))));
                            code.push(ZPAQLOp::Zero(Loc::Reg(Reg::A)));
                            code.push(ZPAQLOp::JT{n: 1});
                            code.push(ZPAQLOp::Inc(Loc::Reg(Reg::A)) );
                        },
                        IROp::Lt => {
                            code.push(ZPAQLOp::Alt(Loc::Reg(Reg::OtherReg(OtherReg::C))) );
                            code.push(ZPAQLOp::Zero(Loc::Reg(Reg::A) ));
                            code.push(ZPAQLOp::JF{n: 1});
                            code.push(ZPAQLOp::Inc(Loc::Reg(Reg::A) ));
                        },
                        IROp::LtE => {
                            code.push(ZPAQLOp::Aeq(Loc::Reg(Reg::OtherReg(OtherReg::C))));
                            code.push(ZPAQLOp::JT{n: 4});
                            code.push(ZPAQLOp::Alt(Loc::Reg(Reg::OtherReg(OtherReg::C))));
                            code.push(ZPAQLOp::Zero(Loc::Reg(Reg::A)));
                            code.push(ZPAQLOp::JF{n: 2});
                            code.push(ZPAQLOp::SetN{target: Loc::Reg(Reg::A), n: 1});
                        },
                        IROp::Gt => {
                            code.push(ZPAQLOp::Agt(Loc::Reg(Reg::OtherReg(OtherReg::C))));
                            code.push(ZPAQLOp::Zero(Loc::Reg(Reg::A)));
                            code.push(ZPAQLOp::JF{n: 1});
                            code.push(ZPAQLOp::Inc(Loc::Reg(Reg::A)));
                        },
                        IROp::GtE => {
                            code.push(ZPAQLOp::Aeq(Loc::Reg(Reg::OtherReg(OtherReg::C))));
                            code.push(ZPAQLOp::JT{n: 4});
                            code.push(ZPAQLOp::Agt(Loc::Reg(Reg::OtherReg(OtherReg::C))));
                            code.push(ZPAQLOp::Zero(Loc::Reg(Reg::A)));
                            code.push(ZPAQLOp::JF{n: 2});
                            code.push(ZPAQLOp::SetN{target: Loc::Reg(Reg::A), n: 1});
                        },
                    }
                    ch.remove_reg(&Loc::Reg(Reg::A));
                    ch.remove_reg(&Loc::Reg(Reg::OtherReg(OtherReg::C)));
                    // assign A to target
                    code.extend_from_slice(&assign_loc_to_var(target, &Loc::Reg(Reg::A), ch));
                }
            },
            IR::MarkTempVarStart | IR::MarkTempVarEnd => {},
            x => { error!("can not emit zpaql for (non-converted?) IR: {}", x); panic!("error"); }
        }
    }
    code
}


/// assign value to location, keeps track in the cache and overwrites A if value>255
fn calc_number(value: u32, loc: &Loc, ch: &mut Cache) -> Vec<ZPAQLOp> {
    match ch.last_hold.get(&loc) {
        Some(&IRVar::Number{value: v}) if v == value => {
            return vec![];
        },
        Some(&IRVar::Number{value: v}) if value > 0 && v == value - 1 => {
            ch.remove_reg(&loc);
            ch.last_hold.insert(loc.clone(), IRVar::Number{value: value});
            return vec![ZPAQLOp::Inc(loc.clone())];
        },
        Some(&IRVar::Number{value: v}) if value < 4294967295 && v == value + 1 => {
            ch.remove_reg(&loc);
            ch.last_hold.insert(loc.clone(), IRVar::Number{value: value});
            return vec![ZPAQLOp::Dec(loc.clone())];
        },
        _ => {},
    }
    let vecc = if value == 4294967295 {
        vec![ZPAQLOp::Zero(loc.clone()), ZPAQLOp::Dec(loc.clone())]
    } else if value == 4294967294 {
        vec![ZPAQLOp::Zero(loc.clone()), ZPAQLOp::Dec(loc.clone()), ZPAQLOp::Dec(loc.clone())]
    } else if value == 2147483648 {
        ch.remove_reg(&Loc::Reg(Reg::A));
        match loc {
            &Loc::Reg(Reg::A) => vec![ZPAQLOp::SetN{target: Loc::Reg(Reg::A), n: 1}, ZPAQLOp::AlshiftN{n: 31}],
            _ => vec![ZPAQLOp::SetN{target: Loc::Reg(Reg::A), n: 1}, ZPAQLOp::AlshiftN{n: 31}, ZPAQLOp::Set{target: loc.clone(), source: Loc::Reg(Reg::A)} ],
        }
    } else if value == 2147483647 {
        ch.remove_reg(&Loc::Reg(Reg::A));
        match loc {
            &Loc::Reg(Reg::A) => vec![ZPAQLOp::Zero(Loc::Reg(Reg::A)), ZPAQLOp::Dec(Loc::Reg(Reg::A)), ZPAQLOp::ArshiftN{n: 1}],
            _ => vec![ZPAQLOp::Zero(Loc::Reg(Reg::A)), ZPAQLOp::Dec(Loc::Reg(Reg::A)), ZPAQLOp::ArshiftN{n: 1}, ZPAQLOp::Set{target: loc.clone(), source: Loc::Reg(Reg::A)}],
        }
    } else if value == 0 {
        vec![ZPAQLOp::Zero(loc.clone())]
    } else if value < 256 {
        vec![ZPAQLOp::SetN{target: loc.clone(), n: value as u8}]
    } else if value < 65536 {
        ch.remove_reg(&Loc::Reg(Reg::A));
        match loc {
            &Loc::Reg(Reg::A) => vec![ZPAQLOp::SetN{target: Loc::Reg(Reg::A), n: (value/256u32) as u8}, ZPAQLOp::AlshiftN{n: 8}, ZPAQLOp::AaddN{n: (value%256u32) as u8}],
            _ => vec![ZPAQLOp::SetN{target: Loc::Reg(Reg::A), n: (value/256u32) as u8}, ZPAQLOp::AlshiftN{n: 8}, ZPAQLOp::AaddN{n: (value%256u32) as u8}, ZPAQLOp::Set{target: loc.clone(), source: Loc::Reg(Reg::A)}],
        }
    } else if value < 16777216 {
        ch.remove_reg(&Loc::Reg(Reg::A));
        match loc {
            &Loc::Reg(Reg::A) => vec![ZPAQLOp::SetN{target: Loc::Reg(Reg::A), n: (value/65536u32) as u8}, ZPAQLOp::AlshiftN{n: 8},
                 ZPAQLOp::AaddN{n: ((value%65536u32)/256u32) as u8},
                 ZPAQLOp::AlshiftN{n: 8}, ZPAQLOp::AaddN{n: (value%256u32) as u8}],
            _ => vec![ZPAQLOp::SetN{target: Loc::Reg(Reg::A), n: (value/65536u32) as u8}, ZPAQLOp::AlshiftN{n: 8},
                 ZPAQLOp::AaddN{n: ((value%65536u32)/256u32) as u8},
                 ZPAQLOp::AlshiftN{n: 8}, ZPAQLOp::AaddN{n: (value%256u32) as u8}, ZPAQLOp::Set{target: loc.clone(), source: Loc::Reg(Reg::A)}],
        }

    } else {
        ch.remove_reg(&Loc::Reg(Reg::A));
        match loc {
            &Loc::Reg(Reg::A) => vec![ZPAQLOp::SetN{target: Loc::Reg(Reg::A), n: (value/16777216u32) as u8}, ZPAQLOp::AlshiftN{n: 8},
                 ZPAQLOp::AaddN{n: ((value%16777216u32)/65536u32) as u8},
                 ZPAQLOp::AlshiftN{n: 8}, ZPAQLOp::AaddN{n: ((value%65536u32)/256u32) as u8},
                 ZPAQLOp::AlshiftN{n: 8}, ZPAQLOp::AaddN{n: (value%256u32) as u8}],
            _ => vec![ZPAQLOp::SetN{target: Loc::Reg(Reg::A), n: (value/16777216u32) as u8}, ZPAQLOp::AlshiftN{n: 8},
                 ZPAQLOp::AaddN{n: ((value%16777216u32)/65536u32) as u8},
                 ZPAQLOp::AlshiftN{n: 8}, ZPAQLOp::AaddN{n: ((value%65536u32)/256u32) as u8},
                 ZPAQLOp::AlshiftN{n: 8}, ZPAQLOp::AaddN{n: (value%256u32) as u8}, ZPAQLOp::Set{target: loc.clone(), source: Loc::Reg(Reg::A)}],
        }
    };
    ch.remove_reg(&loc);
    ch.last_hold.insert(loc.clone(), IRVar::Number{value: value});
    vecc.into_iter().filter(|t| match t { &ZPAQLOp::AaddN{n: 0} => false, _ => true,  } ).collect()
}


/// returns the location of a variable and needed calculations, can overwrite A, C and D, keeps track in the cache
fn gen_loc_for_var(var: &IRVar, ch: &mut Cache) -> (Vec<ZPAQLOp>, Loc) {
    match &(var.tovar()) {
        &IRVar::H{index_varid, orig_name: _} => {
            if ch.is_loc(&Loc::HD, &(var.tovar())) || ch.is_loc(&Loc::Reg(Reg::OtherReg(OtherReg::D)), &IRVar::Var{varid: index_varid}) {
                (vec![], Loc::HD)
            } else {
                ch.last_hold.insert(Loc::Reg(Reg::OtherReg(OtherReg::D)), IRVar::Var{varid: index_varid});
                ch.last_hold.insert(Loc::HD, var.tovar());
                if ch.is_loc(&Loc::Reg(Reg::A), &IRVar::Var{varid: index_varid}){
                    (vec![ZPAQLOp::Set{target: Loc::Reg(Reg::OtherReg(OtherReg::D)), source: Loc::Reg(Reg::A)}], Loc::HD)
                } else {
                    (vec![ZPAQLOp::SetR{target: Reg::OtherReg(OtherReg::D), r: index_varid}], Loc::HD)
                }
            }
        },
        &IRVar::Ht{stack_offset, local, ref orig_name} => {
            if ch.is_loc(&Loc::HD, &(var.tovar())) {
                (vec![], Loc::HD)
            } else if stack_offset > 0 && ch.is_loc(&Loc::HD, &IRVar::Ht{stack_offset: stack_offset-1, local: local, orig_name: orig_name.clone()}) {
                ch.remove_reg(&Loc::Reg(Reg::OtherReg(OtherReg::D)));
                ch.last_hold.insert(Loc::HD, var.tovar());
                (vec![ZPAQLOp::Inc(Loc::Reg(Reg::OtherReg(OtherReg::D)))], Loc::HD)
            } else if ch.is_loc(&Loc::HD, &IRVar::Ht{stack_offset: stack_offset+1, local: local, orig_name: orig_name.clone()}) {
                ch.remove_reg(&Loc::Reg(Reg::OtherReg(OtherReg::D)));
                ch.last_hold.insert(Loc::HD, var.tovar());
                (vec![ZPAQLOp::Dec(Loc::Reg(Reg::OtherReg(OtherReg::D)))], Loc::HD)
            } else {
                let mut v = vec![];
                if !ch.is_loc(&Loc::Reg(Reg::A), &IRVar::Var{varid: if local { 0 } else { 252 }}) {
                    v.push(ZPAQLOp::SetR{target: Reg::A, r: if local { 0 } else { 252 } });
                }
                if stack_offset == 1 {
                    v.push(ZPAQLOp::Inc(Loc::Reg(Reg::A)));
                } else if stack_offset < 256 {
                    v.push(ZPAQLOp::AaddN{n: stack_offset as u8});
                } else {  // @TODO: use calc_number(offset) to add on r0
                    panic!("not implemented")
                }
                v.push(ZPAQLOp::Set{target: Loc::Reg(Reg::OtherReg(OtherReg::D)), source: Loc::Reg(Reg::A)});
                ch.remove_reg(&Loc::Reg(Reg::A));
                ch.remove_reg(&Loc::Reg(Reg::OtherReg(OtherReg::D)));
                ch.last_hold.insert(Loc::HD, var.tovar());
                (v, Loc::HD)
            }
        },
        &IRVar::Hx{addr} => {
            if ch.is_loc(&Loc::Reg(Reg::OtherReg(OtherReg::D)), &IRVar::Number{value: addr}) | ch.is_loc(&Loc::HD, &(var.tovar())) {
                (vec![], Loc::HD)
            } else {
                let v = calc_number(addr, &Loc::Reg(Reg::OtherReg(OtherReg::D)), ch);
                ch.last_hold.insert(Loc::HD, var.tovar());
                (v, Loc::HD)
            }
        },
        &IRVar::Mx{addr} => {
            if ch.is_loc(&Loc::Reg(Reg::OtherReg(OtherReg::C)), &IRVar::Number{value: addr}) | ch.is_loc(&Loc::MC, &(var.tovar())) {
                (vec![], Loc::MC)
            } else {
                let v = calc_number(addr, &Loc::Reg(Reg::OtherReg(OtherReg::C)), ch);
                ch.last_hold.insert(Loc::MC, var.tovar());
                (v, Loc::MC)
            }
        },
        &IRVar::M{index_varid} => {
            if ch.is_loc(&Loc::MC, &(var.tovar())) || ch.is_loc(&Loc::Reg(Reg::OtherReg(OtherReg::C)), &IRVar::Var{varid: index_varid}) {
                (vec![], Loc::MC)
            } else {
                ch.last_hold.insert(Loc::Reg(Reg::OtherReg(OtherReg::C)), IRVar::Var{varid: index_varid});
                ch.last_hold.insert(Loc::MC, var.tovar());
                if ch.is_loc(&Loc::Reg(Reg::A), &IRVar::Var{varid: index_varid}){
                    (vec![ZPAQLOp::Set{target: Loc::Reg(Reg::OtherReg(OtherReg::C)), source: Loc::Reg(Reg::A)}], Loc::MC)
                } else {
                    (vec![ZPAQLOp::SetR{target: Reg::OtherReg(OtherReg::C), r: index_varid}], Loc::MC)
                }
            }
        },
        _ => { error!("no clear location for {}", var); panic!("error") },
    }
}


/// copy value of variable to the location, keeps track in the cache and
/// can overwrite D, B and A on the way, so if variable is on H, loc can't be HD and if variable is on M, loc can't be MB
fn assign_var_to_loc(var: &IRVar, loc: &Loc, ch: &mut Cache) -> Vec<ZPAQLOp> {
    if ch.is_loc(loc, &(var.tovar())) { vec![] } else if ch.is_loc(&Loc::Reg(Reg::A), &(var.tovar())) {
        ch.remove_reg(loc);
        ch.last_hold.insert(loc.clone(), var.tovar());
        vec![ZPAQLOp::Set{target: loc.clone(), source: Loc::Reg(Reg::A)} ]
    } else {
        let v = match &(var.tovar()) {
            &IRVar::Number{value} => { // Big numbers need to be computed
                calc_number(value, loc, ch)
            },
            &IRVar::Var{varid} => {
                match loc {
                    &Loc::Reg(ref reg) => {
                        if ch.is_loc(&Loc::Reg(Reg::A), &(var.tovar())) {
                            ch.last_hold.insert(Loc::Reg(reg.clone()), var.tovar());
                            vec![ZPAQLOp::Set{target: loc.clone(), source: Loc::Reg(Reg::A)} ]
                        } else {
                            ch.last_hold.insert(Loc::Reg(reg.clone()), var.tovar());
                            vec![ZPAQLOp::SetR{target: reg.clone(), r: varid}]
                        }
                    },
                    _ => {
                            if ch.is_loc(&Loc::Reg(Reg::A), &(var.tovar())) {
                                vec![ZPAQLOp::Set{target: loc.clone(), source: Loc::Reg(Reg::A)} ]
                            } else {
                                ch.last_hold.insert(Loc::Reg(Reg::A), var.tovar());
                                vec![ZPAQLOp::SetR{target: Reg::A, r: varid}, ZPAQLOp::Set{target: loc.clone(), source: Loc::Reg(Reg::A)} ]
                            }
                        },
                }
            },
            &IRVar::H{index_varid, orig_name: _} => {
                match loc {
                    &Loc::HD => { error!("Value of D would be overwritten before setting HD") ; panic!("error") },
                    _ => {},
                }
                let mut m = vec![];
                if !ch.is_loc(&Loc::Reg(Reg::OtherReg(OtherReg::D)), &IRVar::Var{varid: index_varid}) && !ch.is_loc(&Loc::HD, &(var.tovar())) {
                    if ch.is_loc(&Loc::Reg(Reg::A), &IRVar::Var{varid: index_varid}){
                        m.push(ZPAQLOp::Set{target: Loc::Reg(Reg::OtherReg(OtherReg::D)), source: Loc::Reg(Reg::A)});
                    } else {
                        m.push(ZPAQLOp::SetR{target: Reg::OtherReg(OtherReg::D), r: index_varid});
                    }
                    ch.last_hold.insert(Loc::Reg(Reg::OtherReg(OtherReg::D)), IRVar::Var{varid: index_varid} );
                    ch.last_hold.insert(Loc::HD, var.tovar());
                }
                m.push(ZPAQLOp::Set{target: loc.clone(), source: Loc::HD});
                m
            },
            &IRVar::Ht{stack_offset, local, ref orig_name} => {
                let mut v = vec![];
                match loc {
                    &Loc::HD => { error!("Value of D would be overwritten before setting HD") ; panic!("error") },
                    _ => {},
                }
                if stack_offset > 0 && ch.is_loc(&Loc::HD, &IRVar::Ht{stack_offset: stack_offset-1, local: local, orig_name: orig_name.clone()}) {
                    ch.remove_reg(&Loc::Reg(Reg::OtherReg(OtherReg::D)));
                    v.push(ZPAQLOp::Inc(Loc::Reg(Reg::OtherReg(OtherReg::D))));
                    ch.last_hold.insert(Loc::HD, var.tovar());
                } else if ch.is_loc(&Loc::HD, &IRVar::Ht{stack_offset: stack_offset+1, local: local, orig_name: orig_name.clone()}) {
                    ch.remove_reg(&Loc::Reg(Reg::OtherReg(OtherReg::D)));
                    v.push(ZPAQLOp::Dec(Loc::Reg(Reg::OtherReg(OtherReg::D))));
                    ch.last_hold.insert(Loc::HD, var.tovar());
                } else if !ch.is_loc(&Loc::HD, &(var.tovar())) {
                    if !ch.is_loc(&Loc::Reg(Reg::A), &IRVar::Var{varid: if local { 0 } else { 252 } }) {
                        v.push(ZPAQLOp::SetR{target: Reg::A, r: if local { 0 } else { 252 } });
                    }
                    if stack_offset == 1 {
                        v.push(ZPAQLOp::Inc(Loc::Reg(Reg::A)));
                    } else if stack_offset < 256 {
                        v.push(ZPAQLOp::AaddN{n: stack_offset as u8});
                    } else {  // @TODO: use calc_number(offset) to add on r0
                        panic!("not implemented")
                    }
                    ch.remove_reg(&Loc::Reg(Reg::A));
                    v.push(ZPAQLOp::Set{target: Loc::Reg(Reg::OtherReg(OtherReg::D)), source: Loc::Reg(Reg::A)});
                    ch.remove_reg(&Loc::Reg(Reg::OtherReg(OtherReg::D)));
                    ch.last_hold.insert(Loc::HD, var.tovar());
                }
                v.push(ZPAQLOp::Set{target: loc.clone(), source: Loc::HD});
                v
            },
            &IRVar::Hx{addr} => {
                let mut v = if ch.is_loc(&Loc::HD, &(var.tovar())) || ch.is_loc(&Loc::Reg(Reg::OtherReg(OtherReg::D)), &IRVar::Number{value: addr}) {
                    vec![]
                } else {
                    calc_number(addr, &Loc::Reg(Reg::OtherReg(OtherReg::D)), ch)
                };
                ch.last_hold.insert(Loc::HD, var.tovar());
                match loc {
                        &Loc::HD => { error!("Value of D would be overwritten before setting HD") ; panic!("error") },
                        _ => {},
                }
                v.push(ZPAQLOp::Set{target: loc.clone(), source: Loc::HD});
                v
            },
            &IRVar::Mx{addr} => {
                let mut v = if ch.is_loc(&Loc::MB, &(var.tovar())) || ch.is_loc(&Loc::Reg(Reg::OtherReg(OtherReg::B)), &IRVar::Number{value: addr}) {
                    vec![]
                } else {
                    calc_number(addr, &Loc::Reg(Reg::OtherReg(OtherReg::B)), ch)
                };
                ch.last_hold.insert(Loc::MB, var.tovar());
                match loc {
                        &Loc::MB => { error!("Value of B would be overwritten before setting MB") ; panic!("error") },
                        _ => {},
                }
                v.push(ZPAQLOp::Set{target: loc.clone(), source: Loc::MB});
                v
            },
            &IRVar::M{index_varid} => {
                match loc {
                        &Loc::MB => { error!("Value of B would be overwritten before setting MB") ; panic!("error") },
                        _ => {},
                }
                let mut m = vec![];
                if !ch.is_loc(&Loc::Reg(Reg::OtherReg(OtherReg::B)), &IRVar::Var{varid: index_varid}) && !ch.is_loc(&Loc::MB, &(var.tovar())) {
                    if ch.is_loc(&Loc::Reg(Reg::A), &IRVar::Var{varid: index_varid}){
                        m.push(ZPAQLOp::Set{target: Loc::Reg(Reg::OtherReg(OtherReg::B)), source: Loc::Reg(Reg::A)});
                    } else {
                        m.push(ZPAQLOp::SetR{target: Reg::OtherReg(OtherReg::B), r: index_varid});
                    }
                    ch.last_hold.insert(Loc::Reg(Reg::OtherReg(OtherReg::B)), IRVar::Var{varid: index_varid} );
                    ch.last_hold.insert(Loc::MB, var.tovar());
                }
                m.push(ZPAQLOp::Set{target: loc.clone(), source: Loc::MB});
                m
            },
            _ => unreachable!(),
        };
        ch.remove_reg(loc);
        ch.last_hold.insert(loc.clone(), var.tovar());
        v
    }
}

// @TODO: maybe sometimes swap (<>) can be used if chache entries exist to to preserve them by swapping back afterwards

/// copy value of location into the location of the variable, keeps track in the cache
/// and can overwrite D, C and A, so if variable is on H, loc can't be HD or D and if variable is on M, loc can't be C or MC
fn assign_loc_to_var(var: &IRVar, loc: &Loc, ch: &mut Cache) -> Vec<ZPAQLOp> {
    if ch.last_hold.get(loc).is_some() && ch.last_hold.get(loc).unwrap() == var {
        return vec![];  // if optimisations are to be turned off, also this case would have to be skipped
    }
    match &(var.tovar()) {
        &IRVar::Number{value: _} => {
            error!("impossible to assign a value to a number"); panic!("error")
        },
        &IRVar::Var{varid} => {
            let mut v = vec![];
            if loc != &Loc::Reg(Reg::A) && !(ch.last_hold.get(&Loc::Reg(Reg::A)).is_some() && ch.last_hold.get(&Loc::Reg(Reg::A)) == ch.last_hold.get(loc)) {
                v.push(ZPAQLOp::Set{target: Loc::Reg(Reg::A), source: loc.clone()});
            }
            v.push(ZPAQLOp::RsetA{n: varid});
            ch.delete(var);
            ch.delete_references(varid);
            ch.last_hold.insert(loc.clone(), var.tovar());
            ch.last_hold.insert(Loc::Reg(Reg::A), var.tovar());
            v
        },
        &IRVar::H{index_varid, orig_name: _} => {
            if loc == &Loc::Reg(Reg::OtherReg(OtherReg::D)) || loc == &Loc::HD {
                error!("would overwrite source"); panic!("error")
            }
            let mut v = vec![];
            if !ch.is_loc(&Loc::Reg(Reg::OtherReg(OtherReg::D)), &IRVar::Var{varid: index_varid}) {
                v.push(ZPAQLOp::SetR{target: Reg::OtherReg(OtherReg::D), r: index_varid});
                ch.last_hold.insert(Loc::Reg(Reg::OtherReg(OtherReg::D)), IRVar::Var{varid: index_varid});
            }
            v.push(ZPAQLOp::Set{target: Loc::HD, source: loc.clone()});
            ch.delete(var);
            ch.last_hold.insert(loc.clone(), var.tovar());
            ch.last_hold.insert(Loc::HD, var.tovar());
            v
        },
        &IRVar::Ht{stack_offset, local, ref orig_name} => {
            let mut v = vec![];
            if loc == &Loc::Reg(Reg::A) {
                v.push(ZPAQLOp::Set{target: Loc::Reg(Reg::OtherReg(OtherReg::C)), source: Loc::Reg(Reg::A)});
                ch.remove_reg(&Loc::Reg(Reg::OtherReg(OtherReg::C)));
            } else if loc == &Loc::Reg(Reg::OtherReg(OtherReg::D)) || loc == &Loc::HD {
                error!("would overwrite source"); panic!("error")
            }
            if stack_offset > 0 && ch.is_loc(&Loc::HD, &IRVar::Ht{stack_offset: stack_offset-1, local: local, orig_name: orig_name.clone()}) {
                ch.remove_reg(&Loc::Reg(Reg::OtherReg(OtherReg::D)));
                v.push(ZPAQLOp::Inc(Loc::Reg(Reg::OtherReg(OtherReg::D))));
            } else if ch.is_loc(&Loc::HD, &IRVar::Ht{stack_offset: stack_offset+1, local: local, orig_name: orig_name.clone()}) {
                ch.remove_reg(&Loc::Reg(Reg::OtherReg(OtherReg::D)));
                v.push(ZPAQLOp::Dec(Loc::Reg(Reg::OtherReg(OtherReg::D))));
            } else if !ch.is_loc(&Loc::HD, &(var.tovar())) {
                if !ch.is_loc(&Loc::Reg(Reg::A), &IRVar::Var{varid: if local {0} else {252} }) {
                    v.push(ZPAQLOp::SetR{target: Reg::A, r: if local {0} else {252} });
                }
                if stack_offset == 1 {
                    v.push(ZPAQLOp::Inc(Loc::Reg(Reg::A)));
                } else if stack_offset < 256 {
                    v.push(ZPAQLOp::AaddN{n: stack_offset as u8});
                } else {  // @TODO: use calc_number(offset) to add on r0
                    panic!("not implemented")
                }
                ch.remove_reg(&Loc::Reg(Reg::A));
                ch.remove_reg(&Loc::Reg(Reg::OtherReg(OtherReg::D)));
                v.push(ZPAQLOp::Set{target: Loc::Reg(Reg::OtherReg(OtherReg::D)), source: Loc::Reg(Reg::A)});
            }
            ch.delete(var);
            ch.last_hold.insert(if loc == &Loc::Reg(Reg::A) { Loc::Reg(Reg::OtherReg(OtherReg::C)) } else {loc.clone()}, var.tovar());
            ch.last_hold.insert(Loc::HD, var.tovar());
            v.push(ZPAQLOp::Set{target: Loc::HD, source: if loc == &Loc::Reg(Reg::A) { Loc::Reg(Reg::OtherReg(OtherReg::C)) } else {loc.clone()} });
            v
        },
        &IRVar::Hx{addr} => {
            let mut v = vec![];
            if loc == &Loc::Reg(Reg::A) {
                v.push(ZPAQLOp::Set{target: Loc::Reg(Reg::OtherReg(OtherReg::C)), source: Loc::Reg(Reg::A)});
                ch.remove_reg(&Loc::Reg(Reg::OtherReg(OtherReg::C)));
            } else if loc == &Loc::Reg(Reg::OtherReg(OtherReg::D)) || loc == &Loc::HD {
                error!("would overwrite source"); panic!("error")
            }
            if !ch.is_loc(&Loc::HD, &(var.tovar())) && !ch.is_loc(&Loc::Reg(Reg::OtherReg(OtherReg::D)), &IRVar::Number{value: addr}) {
                v.extend_from_slice(&calc_number(addr, &Loc::Reg(Reg::OtherReg(OtherReg::D)), ch));
            }
            ch.delete(var);
            ch.last_hold.insert(if loc == &Loc::Reg(Reg::A) { Loc::Reg(Reg::OtherReg(OtherReg::C)) } else {loc.clone()}, var.tovar());
            ch.last_hold.insert(Loc::HD, var.tovar());
            v.push(ZPAQLOp::Set{target: Loc::HD, source: if loc == &Loc::Reg(Reg::A) { Loc::Reg(Reg::OtherReg(OtherReg::C)) } else {loc.clone()} } );
            v
        },
        &IRVar::Mx{addr} => {
            let mut v = vec![];
            if loc == &Loc::Reg(Reg::A) {
                v.push(ZPAQLOp::Set{target: Loc::Reg(Reg::OtherReg(OtherReg::D)), source: Loc::Reg(Reg::A)});
                ch.remove_reg(&Loc::Reg(Reg::OtherReg(OtherReg::D)));
            } else if loc == &Loc::Reg(Reg::OtherReg(OtherReg::C)) || loc == &Loc::MC {
                error!("would overwrite source"); panic!("error")
            }
            if !ch.is_loc(&Loc::MC, &(var.tovar())) && !ch.is_loc(&Loc::Reg(Reg::OtherReg(OtherReg::C)), &IRVar::Number{value: addr}) {
                v.extend_from_slice(&calc_number(addr, &Loc::Reg(Reg::OtherReg(OtherReg::C)), ch));
            }
            ch.delete(var);
            ch.last_hold.insert(if loc == &Loc::Reg(Reg::A) { Loc::Reg(Reg::OtherReg(OtherReg::D)) } else {loc.clone()}, var.tovar());
            ch.last_hold.insert(Loc::MC, var.tovar());
            v.push(ZPAQLOp::Set{target: Loc::MC, source: if loc == &Loc::Reg(Reg::A) { Loc::Reg(Reg::OtherReg(OtherReg::D)) } else {loc.clone()}  });
            v
        },
        &IRVar::M{index_varid} => {
            if loc == &Loc::Reg(Reg::OtherReg(OtherReg::C)) || loc == &Loc::MC {
                error!("would overwrite source"); panic!("error")
            }
            let mut v = vec![];
            if !ch.is_loc(&Loc::Reg(Reg::OtherReg(OtherReg::C)), &IRVar::Var{varid: index_varid}) {
                v.push(ZPAQLOp::SetR{target: Reg::OtherReg(OtherReg::C), r: index_varid});
                ch.last_hold.insert(Loc::Reg(Reg::OtherReg(OtherReg::C)), IRVar::Var{varid: index_varid});
            }
            v.push(ZPAQLOp::Set{target: Loc::MC, source: loc.clone()});
            ch.delete(var);
            ch.last_hold.insert(loc.clone(), var.tovar());
            ch.last_hold.insert(Loc::MC, var.tovar());
            v
        },
        _ => unreachable!(),
    }
}


