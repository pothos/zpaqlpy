use zpaql::{ZPAQLOp, Loc, Reg, OtherReg};
use options;

/// optimise for assignments on VM-arrays:  t1 = t1 + 1; M[t1] = byte; t1 = t1 + 1; M[t1] = byte …
pub fn replace_array_assignment(zcode: Vec<ZPAQLOp>, optioncfg: &options::Options) -> Vec<ZPAQLOp> {
    if optioncfg.no_post_zpaql {
        return zcode;
    }
    let mut code = vec![];
    let mut j: i64 = (zcode.len() as i64) - 1i64;
    let mut ismatch = false;
    while j >= 0 {
        let i = j as usize;
        if i >= 6 {
            match (ismatch, &zcode[i-6], &zcode[i-5], &zcode[i-4], &zcode[i-3], &zcode[i-2], &zcode[i-1], &zcode[i]) {
                //     (cmt)    (cmt)    a++    r=a 1    (cmt)    c=a    *c= 3
                //     (cmt)    (cmt)    a++    r=a 1    (cmt)    c=a    *c= 3
                // becomes:
                //     (cmt) c++      *c= 3
                //     (cmt) c++     *c= 3  a=c   r=a 1
                (false, &ZPAQLOp::Comment{ref comment}, &ZPAQLOp::Comment{comment: _}, &ZPAQLOp::Inc(Loc::Reg(Reg::A)),
                        &ZPAQLOp::RsetA{n: 1}, &ZPAQLOp::Comment{comment: _},
                        &ZPAQLOp::Set{target: Loc::Reg(Reg::OtherReg(OtherReg::C)), source: Loc::Reg(Reg::A)},
                        &ZPAQLOp::SetN{target: Loc::MC, n: value}  ) => {
                        // last occurrence, save to R from C
                        // reverse order for code!
                        code.push(ZPAQLOp::RsetA{n: 1});
                        code.push(ZPAQLOp::Set{target: Loc::Reg(Reg::A), source: Loc::Reg(Reg::OtherReg(OtherReg::C))});
                        code.push(ZPAQLOp::SetN{target: Loc::MC, n: value});
                        code.push(ZPAQLOp::Inc(Loc::Reg(Reg::OtherReg(OtherReg::C))));
                        code.push(ZPAQLOp::Comment{comment: comment.clone()});
                        j -= 6;  // jump over all seven
                        ismatch = true;
                },
                (true, &ZPAQLOp::Comment{ref comment}, &ZPAQLOp::Comment{comment: _}, &ZPAQLOp::Inc(Loc::Reg(Reg::A)),
                        &ZPAQLOp::RsetA{n: 1}, &ZPAQLOp::Comment{comment: _},
                        &ZPAQLOp::Set{target: Loc::Reg(Reg::OtherReg(OtherReg::C)), source: Loc::Reg(Reg::A)},
                        &ZPAQLOp::SetN{target: Loc::MC, n: value}  ) => {
                        // after first occurrence, don't save R again
                        // reverse order for code!
                        code.push(ZPAQLOp::SetN{target: Loc::MC, n: value});
                        code.push(ZPAQLOp::Inc(Loc::Reg(Reg::OtherReg(OtherReg::C))));
                        code.push(ZPAQLOp::Comment{comment: comment.clone()});
                        j -= 6;  // jump over all seven
                        ismatch = true;
                },
                //     (cmt)    (cmt)    a++    r=a 1    (cmt)    c=a    *c=0
                //     (cmt)    (cmt)    a++    r=a 1    (cmt)    c=a    *c=0
                // becomes:
                //     (cmt) c++      *c=0
                //     (cmt) c++     *c=0  a=c   r=a 1
                (false, &ZPAQLOp::Comment{ref comment}, &ZPAQLOp::Comment{comment: _}, &ZPAQLOp::Inc(Loc::Reg(Reg::A)),
                        &ZPAQLOp::RsetA{n: 1}, &ZPAQLOp::Comment{comment: _},
                        &ZPAQLOp::Set{target: Loc::Reg(Reg::OtherReg(OtherReg::C)), source: Loc::Reg(Reg::A)},
                        &ZPAQLOp::Zero(Loc::MC)  ) => {
                        // last occurrence, save to R from C
                        // reverse order for code!
                        code.push(ZPAQLOp::RsetA{n: 1});
                        code.push(ZPAQLOp::Set{target: Loc::Reg(Reg::A), source: Loc::Reg(Reg::OtherReg(OtherReg::C))});
                        code.push(ZPAQLOp::Zero(Loc::MC));
                        code.push(ZPAQLOp::Inc(Loc::Reg(Reg::OtherReg(OtherReg::C))));
                        code.push(ZPAQLOp::Comment{comment: comment.clone()});
                        j -= 6;  // jump over all seven
                        ismatch = true;
                },
                (true, &ZPAQLOp::Comment{ref comment}, &ZPAQLOp::Comment{comment: _}, &ZPAQLOp::Inc(Loc::Reg(Reg::A)),
                        &ZPAQLOp::RsetA{n: 1}, &ZPAQLOp::Comment{comment: _},
                        &ZPAQLOp::Set{target: Loc::Reg(Reg::OtherReg(OtherReg::C)), source: Loc::Reg(Reg::A)},
                        &ZPAQLOp::Zero(Loc::MC)  ) => {
                        // after first occurrence, don't save R again
                        // reverse order for code!
                        code.push(ZPAQLOp::Zero(Loc::MC));
                        code.push(ZPAQLOp::Inc(Loc::Reg(Reg::OtherReg(OtherReg::C))));
                        code.push(ZPAQLOp::Comment{comment: comment.clone()});
                        j -= 6;  // jump over all seven
                        ismatch = true;
                },
                // ### same without comments ### (does not match against three cmds)
                //     a++    r=a 1       c=a    *c= 3
                //     a++    r=a 1      c=a    *c= 3
                // becomes:
                //     c++      *c= 3
                //     c++     *c= 3  a=c   r=a 1
                (false, _, _, _, &ZPAQLOp::Inc(Loc::Reg(Reg::A)),
                        &ZPAQLOp::RsetA{n: 1},
                        &ZPAQLOp::Set{target: Loc::Reg(Reg::OtherReg(OtherReg::C)), source: Loc::Reg(Reg::A)},
                        &ZPAQLOp::SetN{target: Loc::MC, n: value}  ) => {
                        // last occurrence, save to R from C
                        // reverse order for code!
                        code.push(ZPAQLOp::RsetA{n: 1});
                        code.push(ZPAQLOp::Set{target: Loc::Reg(Reg::A), source: Loc::Reg(Reg::OtherReg(OtherReg::C))});
                        code.push(ZPAQLOp::SetN{target: Loc::MC, n: value});
                        code.push(ZPAQLOp::Inc(Loc::Reg(Reg::OtherReg(OtherReg::C))));
                        j -= 3;  // jump over all four
                        ismatch = true;
                },
                (true, _, _, _, &ZPAQLOp::Inc(Loc::Reg(Reg::A)),
                        &ZPAQLOp::RsetA{n: 1},
                        &ZPAQLOp::Set{target: Loc::Reg(Reg::OtherReg(OtherReg::C)), source: Loc::Reg(Reg::A)},
                        &ZPAQLOp::SetN{target: Loc::MC, n: value}  ) => {
                        // after first occurrence, don't save R again
                        // reverse order for code!
                        code.push(ZPAQLOp::SetN{target: Loc::MC, n: value});
                        code.push(ZPAQLOp::Inc(Loc::Reg(Reg::OtherReg(OtherReg::C))));
                        j -= 3;  // jump over all four
                        ismatch = true;
                },
                //     a++    r=a 1      c=a    *c=0
                //     a++    r=a 1      c=a    *c=0
                // becomes:
                //     c++      *c=0
                //     c++     *c=0  a=c   r=a 1
                (false, _, _, _, &ZPAQLOp::Inc(Loc::Reg(Reg::A)),
                        &ZPAQLOp::RsetA{n: 1},
                        &ZPAQLOp::Set{target: Loc::Reg(Reg::OtherReg(OtherReg::C)), source: Loc::Reg(Reg::A)},
                        &ZPAQLOp::Zero(Loc::MC)  ) => {
                        // last occurrence, save to R from C
                        // reverse order for code!
                        code.push(ZPAQLOp::RsetA{n: 1});
                        code.push(ZPAQLOp::Set{target: Loc::Reg(Reg::A), source: Loc::Reg(Reg::OtherReg(OtherReg::C))});
                        code.push(ZPAQLOp::Zero(Loc::MC));
                        code.push(ZPAQLOp::Inc(Loc::Reg(Reg::OtherReg(OtherReg::C))));
                        j -= 3;  // jump over all four
                        ismatch = true;
                },
                (true, _, _, _, &ZPAQLOp::Inc(Loc::Reg(Reg::A)),
                        &ZPAQLOp::RsetA{n: 1},
                        &ZPAQLOp::Set{target: Loc::Reg(Reg::OtherReg(OtherReg::C)), source: Loc::Reg(Reg::A)},
                        &ZPAQLOp::Zero(Loc::MC)  ) => {
                        // after first occurrence, don't save R again
                        // reverse order for code!
                        code.push(ZPAQLOp::Zero(Loc::MC));
                        code.push(ZPAQLOp::Inc(Loc::Reg(Reg::OtherReg(OtherReg::C))));
                        j -= 3;  // jump over all four
                        ismatch = true;
                },
                _ => { code.push(zcode[i].clone()); ismatch = false; }
            }
        } else {
            code.push(zcode[i].clone());
            ismatch = false;
        }
        j -= 1;
    }
    code.reverse();
    code
}

