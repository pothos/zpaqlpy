use zpaqcfg::ZPAQCfgFile;
use zpaql::ZPAQLOp;
use zpaql::ZPAQLOp::*;
use zpaql::Loc;
use zpaql::Loc::{MC, MB, HD};
use zpaql::Reg::{A, OtherReg};
use zpaql::OtherReg::{B, C, D};
use zpaql::SwapLoc;

use std::u32;

// ZPAQL VM for internal testing and exposed (via --run-hcomp) as debugging tool for H[i] values for a file (which is not provided by zpaqd)

pub struct ZPAQLVM {
    pub code: Vec<Option<ZPAQLOp>>,
    pub pc: u16,
    pub h: Vec<u32>,
    pub m: Vec<u8>,
    pub a: u32, pub b: u32, pub c:u32, pub d:u32, pub f: bool,
    pub r: Vec<u32>,
    pub outbuf: Vec<u8>,  // only available in pcomp, ignored in hcomp
}

pub fn zops_to_vmops(ops: &[ZPAQLOp]) -> Vec<Option<ZPAQLOp>> {
    let mut code = vec![];
    for cmd in ops {
        if cmd.size() != 0 {
            code.push(Some(cmd.clone()));
            for _ in 1..cmd.size() {
                code.push(None);  // opcode had more than one byte, therefore this address is invalid
            }
        }
    }
    code
}

impl ZPAQLVM {
    pub fn new(cfgfile: &ZPAQCfgFile) -> (ZPAQLVM, ZPAQLVM) {
        let hh_size: usize = 2u32.pow(cfgfile.hh as u32) as usize;
        let hm_size: usize = 2u32.pow(cfgfile.hm as u32) as usize;
        let ph_size: usize = 2u32.pow(cfgfile.ph as u32) as usize;
        let pm_size: usize = 2u32.pow(cfgfile.pm as u32) as usize;
        let mut hh: Vec<u32> = Vec::with_capacity(hh_size);
        hh.resize(hh_size, 0);
        let mut hm: Vec<u8> = Vec::with_capacity(hm_size);
        hm.resize(hm_size, 0);
        let mut ph: Vec<u32> = Vec::with_capacity(ph_size);
        ph.resize(ph_size, 0);
        let mut pm: Vec<u8> = Vec::with_capacity(pm_size);
        pm.resize(pm_size, 0);
        let mut hr: Vec<u32> = Vec::with_capacity(256);
        hr.resize(256, 0);
        let mut pr: Vec<u32> = Vec::with_capacity(256);
        pr.resize(256, 0);
        let hcomp = zops_to_vmops(&cfgfile.hcomp);
        let pcomp = zops_to_vmops(&cfgfile.pcomp);
        let hcomp_vm = ZPAQLVM{code: hcomp, pc: 0, h: hh, m: hm, r: hr, a: 0, b: 0, c: 0, d: 0, f: false, outbuf: vec![]};
        let pcomp_vm = ZPAQLVM{code: pcomp, pc: 0, h: ph, m: pm, r: pr, a: 0, b: 0, c: 0, d: 0, f: false, outbuf: vec![]};
        (hcomp_vm, pcomp_vm)
    }
    pub fn run(&mut self, c: u32) {
        self.a = c;
        self.pc = 0;
        while self.code[self.pc as usize] != Some(Halt) {
            self.step();
        }
    }
    fn step(&mut self) {
        let pc_opcode = self.pc;
        let opcodebox = self.code[pc_opcode as usize].clone();
        let opcode = opcodebox.unwrap_or_else(
                        || { error!("error: can't execute part of an opcode at {} (invalid jump destination)", self.pc); panic!("error") }
                    );
        self.pc += opcode.size();
        match opcode {
                Error => { error!("Error while running ZPAQL at {}: error instruction", pc_opcode); panic!("error"); },
                Halt => { error!("Error while running ZPAQL at {}: can't execute halt", pc_opcode); panic!("error") },
                Out => { self.outbuf.push(self.a as u8); },
                Hash => { self.a = zmul(zadd(zadd(self.a, self.get_value(&MB)), 512), 773); },
                HashD => {
                    let v = zadd(self.get_value(&HD), self.a);
                    self.set_value(&HD, zmul(zadd(v, 512), 773) ); },
                Inc(ref loc) => {
                    let v = zadd(self.get_value(loc), 1);
                    self.set_value(loc, v); },
                Dec(ref loc) => {
                    let v = zsub(self.get_value(loc), 1);
                    self.set_value(loc, v); },
                Not(ref loc) => {
                    let v = !self.get_value(loc);
                    self.set_value(loc, v); },
                Zero(ref loc) => { self.set_value(loc, 0); },
                Set{ref target, ref source} => {
                    let v = self.get_value(source);
                    self.set_value(target, v); },
                SwapA(ref swaploc) => {
                    match swaploc {
                        &SwapLoc::MB => {
                            let mb = self.get_value(&MB);
                            let v = self.a;
                            self.set_value(&MB, v.clone());
                            self.a = (v & (u32::MAX - 255u32)) + mb; },  // swap only lower 8 bit
                        &SwapLoc::MC => {
                            let mc = self.get_value(&MC);
                            let v = self.a;
                            self.set_value(&MC, v.clone());
                            self.a = (v & (u32::MAX - 255u32)) + mc; },
                        &SwapLoc::HD => {
                            let hd = self.get_value(&HD);
                            let v = self.a;
                            self.set_value(&HD, v);
                            self.a = hd;},
                        &SwapLoc::OtherReg(ref oreg) => {
                            let t = self.get_value(&Loc::Reg(OtherReg(oreg.clone())));
                            let v = self.a;
                            self.set_value(&Loc::Reg(OtherReg(oreg.clone())), v);
                            self.a = t; },
                    }
                },
                SetN{ref target, n} => { self.set_value(&target, n as u32); },
                SetR{ref target, r} => {
                    let rval = self.r[r as usize];
                    self.set_value(&Loc::Reg(target.clone()), rval); },
                Aadd(ref loc) => { self.a = zadd(self.a, self.get_value(loc)); },
                Asub(ref loc) => { self.a = zsub(self.a, self.get_value(loc)); },
                Amult(ref loc) => { self.a = zmul(self.a, self.get_value(loc)); },
                Adiv(ref loc) => { self.a = zdiv(self.a, self.get_value(loc)); },  // &Loc::Reg(A)
                Amod(ref loc) => { self.a = zmod(self.a, self.get_value(loc)); },
                Aand(ref loc) => { self.a = self.a & self.get_value(loc); },
                Aandnot(ref loc) => { self.a = self.a & (!self.get_value(loc)); },
                Aor(ref loc) => { self.a = self.a | self.get_value(loc); },
                Axor(ref loc) => { self.a = self.a ^ self.get_value(loc); },
                Alshift(ref loc) => { self.a = zlshift(self.a, self.get_value(loc)); },
                Arshift(ref loc) => { self.a = zrshift(self.a, self.get_value(loc)); },
                Aeq(ref loc) => { self.f = self.a == self.get_value(loc);  },
                Alt(ref loc) => { self.f = self.a < self.get_value(loc);  },
                Agt(ref loc) => { self.f = self.a > self.get_value(loc);  },

                JT{n} => if self.f { self.pc = (self.pc as i32 + n as i32) as u16; } ,  // PCnextInstr += n (signed)    in bytecode N is positive: ((N+128) mod 256) - 128
                JF{n} => if !self.f { self.pc = (self.pc as i32 + n as i32) as u16; } ,
                JMP{n} => { self.pc = (self.pc as i32 + n as i32) as u16;  },
                RsetA{n} => { self.r[n as usize] = self.a;  },
                AaddN{n} => { self.a = zadd(self.a, n as u32); },
                AsubN{n} => { self.a = zsub(self.a, n as u32); },
                AmultN{n} => { self.a = zmul(self.a, n as u32); },
                AdivN{n} => { self.a = zdiv(self.a, n as u32); },
                AmodN{n} => { self.a = zmod(self.a, n as u32); },
                AandN{n} => { self.a = self.a & (n as u32); },
                AandnotN{n} => { self.a = self.a & (!(n as u32)); },
                AorN{n} => { self.a = self.a | (n as u32); },
                AxorN{n} => { self.a = self.a ^ (n as u32); },
                AlshiftN{n} => { self.a = zlshift(self.a, n as u32); },
                ArshiftN{n} => { self.a = zrshift(self.a, n as u32); },
                AeqN{n} => { self.f = self.a == (n as u32);  },
                AltN{n} => { self.f = self.a < (n as u32);  },
                AgtN{n} => { self.f = self.a > (n as u32); },

                LJ{n} => { self.pc = n; },  // jump to n, operands as bytecode PC := 256 * M + N

                ref cmd => { panic!("can't execute {}: {}", pc_opcode, cmd); },
        }
    }
    fn get_value(&self, loc: &Loc) -> u32 {
        match loc {
            &Loc::Reg(A) => self.a,
            &Loc::Reg(OtherReg(B)) => self.b,
            &Loc::Reg(OtherReg(C)) => self.c,
            &Loc::Reg(OtherReg(D)) => self.d,
            &MB => self.m[(self.b as usize % self.m.len()) as usize] as u32,
            &MC => self.m[(self.c as usize % self.m.len()) as usize] as u32,
            &HD => self.h[(self.d as usize % self.h.len()) as usize],
        }
    }
    fn set_value(&mut self, loc: &Loc, v: u32) {
        match loc {
            &Loc::Reg(A) => { self.a = v; },
            &Loc::Reg(OtherReg(B)) => { self.b = v; },
            &Loc::Reg(OtherReg(C)) => { self.c = v; },
            &Loc::Reg(OtherReg(D)) => { self.d = v; },
            &MB => {
                let leng = self.m.len();
                self.m[(self.b as usize % leng) as usize] = v as u8; },  // use only least 8 bit
            &MC => {
                let leng = self.m.len();
                self.m[(self.c as usize % leng) as usize] = v as u8; },
            &HD => {
                let leng = self.h.len();
                self.h[(self.d as usize % leng) as usize] = v; },
        }
    }
}

fn zadd(x: u32, y: u32) -> u32 {
    let (res, _) = x.overflowing_add(y);
    res
}

fn zsub(x: u32, y: u32) -> u32 {
    let (res, _) = x.overflowing_sub(y);
    res
}

fn zmul(x: u32, y: u32) -> u32 {
    let (res, _) = x.overflowing_mul(y);
    res
}

fn zdiv(x: u32, y: u32) -> u32 {
    if y == 0 {
        0
    } else {
        let (res, _) = x.overflowing_div(y);
        res
    }
}

fn zmod(x: u32, y: u32) -> u32 {
    if y == 0 {
        0
    } else {
        let (res, _) = x.overflowing_rem(y);
        res
    }
}

fn zlshift(x: u32, y: u32) -> u32 {
    let (res, _) = x.overflowing_shl(y);
    res
}

fn zrshift(x: u32, y: u32) -> u32 {
    let (res, _) = x.overflowing_shr(y);
    res
}

