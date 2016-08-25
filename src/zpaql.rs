use std::collections::{HashMap};
use std::fmt::{Display, Formatter, Error};
use options;

// @TODO: try traits instead of the nested enums
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Reg {
    A,
    OtherReg(OtherReg),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OtherReg {
    B,
    C,
    D,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Loc {
    Reg(Reg),
    MB,
    MC,
    HD,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum SwapLoc { // with A
    OtherReg(OtherReg),
    MB, // only low 8 bits of A are touched
    MC, // only low 8 bits of A are touched
    HD,
}


impl Display for Reg {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), Error> {
        match *self {
            Reg::A => { write!(fmt, "a") },
            Reg::OtherReg(ref r) => { write!(fmt, "{}", r) },
        }
    }
}

impl Display for OtherReg {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), Error> {
        match *self {
            OtherReg::B => { write!(fmt, "b") },
            OtherReg::C => { write!(fmt, "c") },
            OtherReg::D => { write!(fmt, "d") },
        }
    }
}

impl Display for Loc {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), Error> {
        match *self {
            Loc::MB => { write!(fmt, "*b") },
            Loc::MC => { write!(fmt, "*c") },
            Loc::HD => { write!(fmt, "*d") },
            Loc::Reg(ref r) => { write!(fmt, "{}", r) },
        }
    }
}

impl Display for SwapLoc {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), Error> {
        match *self {
            SwapLoc::MB => { write!(fmt, "*b") },
            SwapLoc::MC => { write!(fmt, "*c") },
            SwapLoc::HD => { write!(fmt, "*d") },
            SwapLoc::OtherReg(ref r) => { write!(fmt, "{}", r) },
        }
    }
}


#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum ZPAQLOp {  // please extend match in .size() after changes here
    Error,
    Halt,
    Out,
    Hash,  // A := (A + M[B] + 512) * 773
    HashD,  // H[D] := (H[D] + A + 512) * 773

    Inc(Loc),
    Dec(Loc),
    Not(Loc),
    Zero(Loc),
    Set{target: Loc, source: Loc},
    SwapA(SwapLoc),
    SetN{target: Loc, n: u8},  // 2 byte opcode
    SetR{target: Reg, r: u8}, // 2 byte opcode
    Aadd(Loc),
    Asub(Loc),
    Amult(Loc),
    Adiv(Loc),
    Amod(Loc),
    Aand(Loc),
    Aandnot(Loc),
    Aor(Loc),
    Axor(Loc),
    Alshift(Loc),
    Arshift(Loc),
    Aeq(Loc), // sets F
    Alt(Loc), // sets F
    Agt(Loc), // sets F

    JT{n: i8},  // PCnextInstr += n (signed)    in bytecode N is positive: ((N+128) mod 256) - 128    // 2 byte opcode
    JF{n: i8}, // 2 byte opcode
    JMP{n: i8}, // 2 byte opcode
    RsetA{n: u8}, // 2 byte opcode
    AaddN{n: u8}, // 2 byte opcode
    AsubN{n: u8}, // 2 byte opcode
    AmultN{n: u8}, // 2 byte opcode
    AdivN{n: u8}, // 2 byte opcode
    AmodN{n: u8}, // 2 byte opcode
    AandN{n: u8}, // 2 byte opcode
    AandnotN{n: u8}, // 2 byte opcode
    AorN{n: u8}, // 2 byte opcode
    AxorN{n: u8}, // 2 byte opcode
    AlshiftN{n: u8}, // 2 byte opcode
    ArshiftN{n: u8}, // 2 byte opcode
    AeqN{n: u8},  // sets F  // 2 byte opcode
    AltN{n: u8}, // 2 byte opcode
    AgtN{n: u8}, // 2 byte opcode

    LJ{n: u16},  // jump to n, operands as bytecode PC := 256 * M + N    // 3 byte opcode

    Comment{comment: String}, // 0 byte opcode
    Label{label: String, position: u32},  // 0 byte opcode
    GoTo{label: String},  // virtual 3 byte opcode, becomes LJ
}  // please extend match in .size() after changes here

/// convert goto helper instruction to long jumps
pub fn set_positions(code: &[ZPAQLOp], optioncfg: &options::Options) -> Vec<ZPAQLOp> {
    let mut pos: u16 = 0;
    let mut ops = vec![];
    let mut labels: HashMap<String, u16> = HashMap::<String, u16>::new();
    for instr in code {
        match instr {
            &ZPAQLOp::Label{ref label, position: _} => {
                labels.insert(label.clone(), pos);
                },
            i => {
                let new_pos: u32 = pos as u32 + i.size() as u32;
                if new_pos > 65535 {
                    error!("zpaql file gets too big with instruction at {} (only 64k are allowed)", new_pos);
                    if !optioncfg.ignore_errors {
                        panic!("error");
                    }
                }
                pos = new_pos as u16;
            },
        }
    }
    for instr in code {
        match instr {
            &ZPAQLOp::Label{label: _, position: _} => {
                // ops.push(ZPAQLOp::Comment{comment: format!("{}:", label)});
            },
            &ZPAQLOp::GoTo{ref label} => {
                let posi = labels.get(label).unwrap_or_else(|| { error!("label {} not found", label); panic!("error") } );
                // ops.push(ZPAQLOp::Comment{comment: format!("goto {}", label)});
                ops.push(ZPAQLOp::LJ{n: *posi});
            },
            i => { ops.push(i.clone()); },
        }
    }
    ops
}

impl ZPAQLOp {

    /// opcode size in bytes, please extend match if you add helper meta opcodes
    pub fn size(&self) -> u16 {
        use self::ZPAQLOp::*;
        match *self {
            Comment{comment: _} => 0,
            Label{label: _, position: _} => 0,

            JT{n: _} | JF{n: _} | JMP{n: _} | RsetA{n: _} | AaddN{n: _}
            | AsubN{n: _} | AmultN{n: _} | AdivN{n: _} | AmodN{n: _} | AandN{n: _}
            | AandnotN{n: _} | AorN{n: _} | AxorN{n: _} | AlshiftN{n: _} | ArshiftN{n: _}
            | AeqN{n: _} | AltN{n: _} | AgtN{n: _} => 2,

            LJ{n: _} => 3,
            GoTo{label: _} => 3, // virtual instruction, becomes LJ

            Inc(_) | Dec(_) | Not(_) | Zero(_) | Set{target: _, source: _}
            | SwapA(_) | Aadd(_) | Asub(_) | Amult(_) | Adiv(_) | Amod(_) | Aand(_) | Aandnot(_)
            | Aor(_) | Axor(_) | Alshift(_) | Arshift(_) | Aeq(_) | Alt(_) | Agt(_) => 1,

            SetN{target: _, n: _} | SetR{target: _, r: _} => 2,

            Error | Halt | Out | Hash | HashD => 1,
        }
    }
}


impl Display for ZPAQLOp {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), Error> {
        use self::ZPAQLOp::*;
        match *self {
            Label{ref label, position} => { write!(fmt, "{}: {}", label, position) },
            GoTo{ref label} => { write!(fmt, "goto {}", label) },
            Error => { write!(fmt, "error") },
            Halt => { write!(fmt, "halt") },
            Out => { write!(fmt, "out") },
            Hash => { write!(fmt, "hash") },
            HashD => { write!(fmt, "hashd") },

            Inc(ref l) => { write!(fmt, "{}++", l) },
            Dec(ref l) => { write!(fmt, "{}--", l) },
            Not(ref l) => { write!(fmt, "{}!", l) },
            Zero(ref l) => { write!(fmt, "{}=0", l) },
            Set{ref target, ref source} => { write!(fmt, "{}={}", target, source) },
            SwapA(ref l) => { write!(fmt, "{}<>a", l) },
            SetN{ref target, n} => { write!(fmt, "{}= {}", target, n) },
            SetR{ref target, r} => { write!(fmt, "{}=r {}", target, r) },
            Aadd(ref l) => { write!(fmt, "a+={}", l) },
            Asub(ref l) => { write!(fmt, "a-={}", l) },
            Amult(ref l) => { write!(fmt, "a*={}", l) },
            Adiv(ref l) => { write!(fmt, "a/={}", l) },
            Amod(ref l) => { write!(fmt, "a%={}", l) },
            Aand(ref l) => { write!(fmt, "a&={}", l) },
            Aandnot(ref l) => { write!(fmt, "a&~{}", l) },
            Aor(ref l) => { write!(fmt, "a|={}", l) },
            Axor(ref l) => { write!(fmt, "a^={}", l) },
            Alshift(ref l) => { write!(fmt, "a<<={}", l) },
            Arshift(ref l) => { write!(fmt, "a>>={}", l) },
            Aeq(ref l) => { write!(fmt, "a=={}", l) },
            Alt(ref l) => { write!(fmt, "a<{}", l) },
            Agt(ref l) => { write!(fmt, "a>{}", l) },


            JT{n} => { write!(fmt, "jt {}", n) },
            JF{n} => { write!(fmt, "jf {}", n) },
            JMP{n} => { write!(fmt, "jmp {}", n) },
            RsetA{n} => { write!(fmt, "r=a {}", n) },

            AaddN{n} => { write!(fmt, "a+= {}", n) },
            AsubN{n} => { write!(fmt, "a-= {}", n) },
            AmultN{n} => { write!(fmt, "a*= {}", n) },
            AdivN{n} => { write!(fmt, "a/= {}", n) },
            AmodN{n} => { write!(fmt, "a%= {}", n) },
            AandN{n} => { write!(fmt, "a&= {}", n) },
            AandnotN{n} => { write!(fmt, "a&~ {}", n) },
            AorN{n} => { write!(fmt, "a|= {}", n) },
            AxorN{n} => { write!(fmt, "a^= {}", n) },
            AlshiftN{n} => { write!(fmt, "a<<= {}", n) },
            ArshiftN{n} => { write!(fmt, "a>>= {}", n) },
            AeqN{n} => { write!(fmt, "a== {}", n) },
            AltN{n} => { write!(fmt, "a< {}", n) },
            AgtN{n} => { write!(fmt, "a> {}", n) },

            LJ{n} => { write!(fmt, "lj {}", n) },

            Comment{ref comment} => { write!(fmt, "({})", comment.replace("(", "〈").replace(")", "〉") ) },

        }
    }
}


