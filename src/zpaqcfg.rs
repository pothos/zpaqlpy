use std::io::Write;
use std::fs::File;
use zpaql::{ZPAQLOp, set_positions};
use options;

pub struct ZPAQCfgFile {
    pub hh: u8,
    pub hm: u8,
    pub ph: u8,
    pub pm: u8,
    pub n: u8, // == model.len()
    pub model: Vec<(u8, String)>,  // ? maybe data type instead of string
    pub pcomp_invocation: String,
    pub stacksize: u32,  // <= 2^32 - 2^?h
    pub hcomp: Vec<ZPAQLOp>,
    pub pcomp: Vec<ZPAQLOp>,
    pub finalised: bool,
}

fn calc_xh_size(hlog: u8, stacksize: u32, optioncfg: &options::Options) -> u8 {
    // using the formular log(x+y) = log(x) + log(1 + y/x)
    let nhlog: f32 = (hlog as f32 + (1f32 + (stacksize as f32)/2f32.powi(hlog as i32)).log2() ).ceil();
    if nhlog > 32f32 {
        error!("size of H is too big: **2^{}** = {} + 2^{} = stacksize + 2^?h <= 2^32", stacksize, hlog, nhlog as u64);
        if !optioncfg.ignore_errors{
            panic!("error");
        }
    }
    nhlog as u8
}

impl ZPAQCfgFile {
    pub fn finalise(&mut self, optioncfg: &options::Options) -> Result<(), ()> {
        if self.finalised {
            Err(())
        } else {
            let total_hh = calc_xh_size(self.hh, if self.hcomp.is_empty() || self.n == 0 {0} else {self.stacksize}, optioncfg);
            let total_ph = calc_xh_size(self.ph, if self.pcomp.is_empty() {0} else {self.stacksize}, optioncfg);
            self.hh = total_hh;
            self.ph = total_ph;
            self.hcomp = set_positions(&self.hcomp, optioncfg);
            self.pcomp = set_positions(&self.pcomp, optioncfg);
            self.finalised = true;
            Ok(())
        }
    }
    pub fn write_header(&self, mut output: &File) {
        assert!(self.finalised);
        write!(output, "comp {} {} {} {} {} (hh hm ph pm n)\n",
                self.hh,
                self.hm,
                self.ph,
                self.pm, self.n).unwrap();
        if self.n > 0 {
            for &(i, ref c) in &self.model {
                write!(output, "  {} {}\n", i, c).unwrap();
            }
        }
    }
    pub fn write_hcomp(&self, mut output: &File, optioncfg: &options::Options) {
        assert!(self.finalised);
        write!(output, "hcomp\n").unwrap();
        if self.hcomp.is_empty() || self.n == 0 {
            write!(output, "  halt\n").unwrap();
        } else {
            let mut pc = 0;
            for zpaqlop in &self.hcomp {
                if optioncfg.pc_as_comment {
                    write!(output, "  {}        ({})\n", zpaqlop, pc).unwrap();
                } else {
                    write!(output, "  {}\n", zpaqlop).unwrap();
                }
                pc += zpaqlop.size();
            }
        }
    }
    pub fn write_pcomp(&self, mut output: &File, optioncfg: &options::Options) {
        assert!(self.finalised);
        if !self.pcomp.is_empty() {
            write!(output, "pcomp {} ;\n", self.pcomp_invocation).unwrap();
            let mut pc = 0;
            for zpaqlop in &self.pcomp {
                if optioncfg.pc_as_comment {
                    write!(output, "  {}        ({})\n", zpaqlop, pc).unwrap();
                } else {
                    write!(output, "  {}\n", zpaqlop).unwrap();
                }
                pc += zpaqlop.size();
            }
        }
    }
    pub fn write_end(&self, mut output: &File) {
        assert!(self.finalised);
        write!(output, "end\n").unwrap();
    }
}

