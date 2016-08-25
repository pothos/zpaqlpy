extern crate lalrpop;
use std::process::Command;
use std::fs;

fn main() {
    let s = Command::new("lalrpop").arg("src/grammar.lalrpop").status();
    if s.is_err() {
        lalrpop::process_root().unwrap();
    } else if !s.unwrap().success() {
        error!("external lalrpop failed");
        lalrpop::process_root().unwrap();
    }
}
