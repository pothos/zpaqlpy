extern crate lalrpop;
use std::process::Command;

fn main() {
    let s = Command::new("lalrpop").arg("src/grammar.lalrpop").status();
    if s.is_err() {
        lalrpop::process_root().unwrap();
    } else if !s.unwrap().success() {
        println!("external lalrpop failed");
        lalrpop::process_root().unwrap();
    }
}
