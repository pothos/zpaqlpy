#[macro_use]
extern crate log;
extern crate flexi_logger;
#[macro_use]
extern crate clap;
extern crate regex;

pub mod grammar; // synthesized by LALRPOP
mod tok;
mod ast;
mod gen_ir;
mod zpaqcfg;
mod zpaql;
mod gen_zpaql;
mod post_zpaql;
mod ir;
mod template;
mod documentation;
mod options;
mod rtok;
mod post_ir;
mod zpaqlvm;

use flexi_logger::{init,LogConfig};
use std::io::{Read,Write};
use std::fs::File;
use std::process::exit;
use std::panic;
use std::str::FromStr;
use std::collections::HashMap;

fn main() {
    let mut optioncfg = options::Options::new();
    let matches = clap::App::new("zpaqlpy compiler")
                    .version(crate_version!())
                    .about("Compile a zpaqlpy source file to a ZPAQ configuration file for usage with zpaqd
Copyright (C) 2016 Kai Lüke kailueke@riseup.net
This program comes with ABSOLUTELY NO WARRANTY and is free software, you are welcome to redistribute it
under certain conditions, see https://www.gnu.org/licenses/gpl-3.0.en.html")
                    .args_from_usage(
                              "-o, --output=[FILE] 'Set the output file (default: INPUT with suffix .cfg, - for stdout is not supported)'
                              [INPUT]              'Set the input file (- for stdin), must be a valid Python file in form of the template (UTF-8, no BOM)'
                              -v...                'Set the level of verbosity (-v: warn, -vv: info, -vvv: debug, -vvvv: trace)'
                              --info-zpaq                        'Show information on the ZPAQ standard'
                              --info-zpaql                       'Show information on the ZPAQL target language'
                              --info-zpaqlpy                     'Show information on the supported Python-subset (zpaqlpy)'
                              --info-zpaqlir                     'Show information on the used intermediate representation language (zpaqlir)'
                              --info-tutorial                    'Show a small tutorial'
                              -S                                 'Write only intermediate representation code to output (suffix for default gets INPUT.ir)'
                              --suppress-pcomp                   'Behave as if \"def pcomp(): pass\" is present, emit an empty pcomp section'
                              --suppress-hcomp                   'Behave as if \"def hcomp(): pass\" is present, emit an empty hcomp section'
                              --disable-comp                     'No context-mixing components and arithmetic coding, also suppress hcomp'
                              --disable-optim                    'Disable lifetime optimisation passes'
                              --fixed-global-access              'Calculate full address for globals on each access'
                              --ignore-errors                    'Continues for some errors which lead to an invalid ZPAQ config file'
                              --emit-template                    'Print an empty template (supports -o)'
                              --no-post-zpaql                    'Disable ZPAQL optimisation pass for successive byte assignments on an array in M'
                              --no-comments                      'Do not write original code lines as comments beside output'
                              --no-pc-comments                   'Do not annotate programme counter for opcodes'
                              --stacksize=[NUMBER]            'Set size of stack to NUMBER (default: 1048576 = 1MiB, <= 2^32 - 2^?h)'
                              --extern-tokenizer                 'Use python3 -m tokenize -e instead of internal tokenizer'
                              --run-hcomp=[FILE]                 'Execute the resulting cfg file like \"zpaqd r CFG h FILE\" and print H[0]…H[n-1]'
                              --notemp_debug_cfg                 'Disable temporary new feature'"
                    ).get_matches();
    if matches.is_present("info-zpaq") {
        println!("{}", documentation::INFO_ZPAQ);
        return;
    }
    if matches.is_present("info-zpaql") {
        println!("{}", documentation::INFO_ZPAQL);
        return;
    }
    if matches.is_present("info-zpaqlpy") {
        println!("{}", documentation::INFO_ZPAQLPY);
        return;
    }
    if matches.is_present("info-zpaqlir") {
        println!("{}", documentation::INFO_ZPAQLIR);
        return;
    }
    if matches.is_present("info-tutorial") {
        println!("{}", documentation::INFO_TUTORIAL);
        return;
    }
    // please keep in sync with defaults in options.rs
    optioncfg.suppress_pcomp = matches.is_present("suppress-pcomp");
    optioncfg.suppress_hcomp = matches.is_present("suppress-hcomp");
    optioncfg.disable_comp = matches.is_present("disable-comp");
    optioncfg.disable_optim = matches.is_present("disable-optim");
    optioncfg.ignore_errors = matches.is_present("ignore-errors");
    optioncfg.fixed_global_access = matches.is_present("fixed-global-access");
    optioncfg.temp_debug_cfg = !matches.is_present("notemp_debug_cfg");
    optioncfg.emit_ir = matches.is_present("S");
    optioncfg.extern_tokenizer = matches.is_present("extern-tokenizer");
    optioncfg.comments = !matches.is_present("no-comments");
    optioncfg.no_post_zpaql = matches.is_present("no-post-zpaql");
    optioncfg.pc_as_comment = !matches.is_present("no-pc-comments");
    let log_level = match matches.occurrences_of("v") {
        0 => "error",
        1 => "warn",
        2 => "info",
        3 => "debug",
        4 | _ => "trace",
    };
    init(LogConfig::new(), Some(log_level.to_string())).unwrap();
    if matches.is_present("stacksize") {
        optioncfg.stacksize = u32::from_str(matches.value_of("stacksize").unwrap()).unwrap_or_else(|e| {
            error!("stacksize must be a number: {}", e);
            panic!("error") });
    }

    // write out an empty python template source file and quit
    if matches.is_present("emit-template") {
        if matches.is_present("output") {
            let template_name = matches.value_of("output").unwrap().to_string();
            let mut template_file = std::fs::OpenOptions::new().write(true).truncate(true).create(true).open(
                    &std::path::Path::new(&template_name[..])
            ).unwrap_or_else(
                        |e| { error!("Could not create {}: {}", template_name, e); exit(3) }
            );
            write!(template_file, "{}", template::EMPTY_SOURCE).unwrap();
        } else {
            println!("{}", template::EMPTY_SOURCE);
        }
        return;
    }
    let mut input = String::new(); // content of source file
    match matches.value_of("INPUT").unwrap_or_else(|| { error!("No input file specified. Invoke with --help or -h to see usage."); exit(1) } ) {
        "-" => {
            {
            let stdin = std::io::stdin();
            stdin.lock().read_to_string(&mut input).unwrap();
            }
        },
        filename => {
            std::fs::File::open(&std::path::Path::new(filename)).unwrap_or_else(
                    |e| { error!("Could not open {}: {}", filename, e); exit(2) }
                ).read_to_string(&mut input).unwrap();
        },
    };
    let outname = if matches.is_present("output") {
        matches.value_of("output").unwrap().to_string()
    } else {
        let mut inp = matches.value_of("INPUT").unwrap_or("out.py").to_string();
        if inp == "-" { inp = "out.py".to_string(); }
        if inp.ends_with(".py") {
            inp.pop(); inp.pop();
        }
        if optioncfg.emit_ir {
            inp.push_str("ir");
        } else {
            inp.push_str("cfg");
        }
        inp
    };
    // create output file
    let output = std::fs::OpenOptions::new().write(true).truncate(true).create(true).open(&std::path::Path::new(&outname[..])).unwrap_or_else(
            |e| { error!("Could not create {}: {}", outname, e); exit(3) }
        );
    // start compiler
    let zcfgfile = compile(&optioncfg, input, output);
    if matches.is_present("run-hcomp") && zcfgfile.is_some() {  // support debugging of computation in hcomp like the python script
        let hinput = matches.value_of("run-hcomp").unwrap();
        let zcfg = zcfgfile.unwrap();
        let (mut hvm, _) = zpaqlvm::ZPAQLVM::new(&zcfg);
        for byte in std::fs::File::open(&std::path::Path::new(hinput)).unwrap_or_else(
                        |e| { error!("Could not open {}: {}", hinput, e); exit(2) }
                    ).bytes() {
            let b = byte.unwrap();
            hvm.run(b as u32);
            println!("{}: {:?}", b, &hvm.h[0..zcfg.n as usize]);
        }
    }
}

/// parses the input source string (which must be based on a template and it's conditions)
/// according to the options, by first tokenizing the input and then spliting up the sections
/// before parsing, prefixed with the common code in comp-section
/// and give back the AST for these sections, both starting with the same comp-code
/// (see tok::seperate_sections)
fn parse(optioncfg: &options::Options, input: &String) -> (Vec<ast::Stmt>, Vec<ast::Stmt>) {
    let tokens = if optioncfg.extern_tokenizer { // external tokenizer is requested
        let tokens_extern = tok::tokenize(input); // calls python -m tokenize -e
        let result = panic::catch_unwind(|| {
            let tokens  = rtok::tokenize(input); // compare output with internal tokenizer
            for (t, te) in tokens.iter().zip(tokens_extern.iter()) {
                if t != te {
                    error!("tokens differ (intern, extern): {:?} ←→ {:?}", t, te);
                }
            }
        });
        if let Err(_) = result { // Err(err)
            error!("internal tokenizer failed on input, run without external tokenizer to see it's error message");
            // panic::resume_unwind(err);
        }
        tokens_extern
    } else {
        rtok::tokenize(input) // use internal tokenizer only
    };
    let (hcomp, pcomp) = tok::seperate_sections(tokens);
    info!("extracted section hcomp:");
    for tokn in &hcomp {
        debug!("  {:?},", tokn);
    }
    info!("extracted section pcomp:");
    for tokn in &pcomp {
        debug!("  {:?},", tokn);
    }
    info!("end of extracted sections");
    let parsed_hcomp = grammar::ProgParser::new().parse(hcomp).unwrap_or_else(|e| panic!("parser error: {:?}", e) );
    info!("parsed grammar hcomp");
    debug!("[\n  {}]", parsed_hcomp.iter().map(|st| format!("{}", st).replace("\n", "\n  ")).collect::<Vec<String>>()[..].join(",\n  "));
    let parsed_pcomp = grammar::ProgParser::new().parse(pcomp).unwrap();
    info!("parsed grammar pcomp");
    debug!("[\n  {}]", parsed_pcomp.iter().map(|st| format!("{}", st).replace("\n", "\n  ")).collect::<Vec<String>>()[..].join(",\n  "));
    (parsed_hcomp, parsed_pcomp)
}

fn xcomp_ir_string(convert: bool, xcomp_ir: &[ir::IR]) -> String { // expand and break down meta-instructions
    xcomp_ir.iter().map(|st| format!("{}", if convert {st.convert()} else {st.clone()})).collect::<Vec<String>>()[..].join("\n")
}

/// compile ASTs to IR code for hcomp and pcomp and read in the comp-section to zpaqcfgfile
fn build_ir(optioncfg: &options::Options, parsed_hcomp: Vec<ast::Stmt>, parsed_pcomp: Vec<ast::Stmt>, input: String) -> (zpaqcfg::ZPAQCfgFile, Vec<ir::IR>, Vec<ir::IR>) {
    // the first 6 assignments contain the values for ph, pm, hh, hm, n and pcomp_invocation
    let zpaqcfgfile = gen_ir::read_context_model(&parsed_pcomp[..6], optioncfg);
    info!("generate IR for hcomp");
    let mut hcomp_ir = gen_ir::gen_code(true, &parsed_hcomp[6..], &zpaqcfgfile, input.clone(), optioncfg);
    hcomp_ir = post_ir::optimise(hcomp_ir, optioncfg);
    debug!("\n{}", xcomp_ir_string(false, &hcomp_ir[..]));
    info!("generate IR for pcomp");
    let mut pcomp_ir = gen_ir::gen_code(false, &parsed_pcomp[6..], &zpaqcfgfile, input, optioncfg);
    pcomp_ir = post_ir::optimise(pcomp_ir, optioncfg);
    debug!("\n{}", xcomp_ir_string(false, &pcomp_ir[..]));
    (zpaqcfgfile, hcomp_ir, pcomp_ir)
}

/// compile input source file and write a ZPAQ configuration to output, following options as specified
fn compile(optioncfg: &options::Options, input: String, mut output: File) -> Option<zpaqcfg::ZPAQCfgFile> {
    let (parsed_hcomp, parsed_pcomp) = parse(optioncfg, &input);
    let (mut zpaqcfgfile, hcomp_ir, pcomp_ir) = build_ir(optioncfg, parsed_hcomp, parsed_pcomp, input);
    if optioncfg.disable_comp { // suppress usage of context-mixing model
        zpaqcfgfile.n = 0;
    }
    if optioncfg.emit_ir { // do not write out compiled ZPAQL code to file but IR code
        info!("write out IR cfg file");
        if !hcomp_ir.is_empty() && !optioncfg.suppress_hcomp && zpaqcfgfile.n > 0 {
            zpaqcfgfile.hcomp = vec![zpaql::ZPAQLOp::Halt]; // fill with dummy content
        }
        if !pcomp_ir.is_empty() && !optioncfg.suppress_pcomp {
            zpaqcfgfile.pcomp = vec![zpaql::ZPAQLOp::Halt]; // fill with dummy content
        }
        zpaqcfgfile.finalise(optioncfg).unwrap();
        zpaqcfgfile.write_header(&output);
        write!(output, "hcomp\n").unwrap();  // similar implementation as .write_hcomp and .write_pcomp but for IR
        if !zpaqcfgfile.hcomp.is_empty() {
            info!("emit IR for hcomp");
            write!(output, "{}\n", xcomp_ir_string(true, &hcomp_ir[..])).unwrap();
        }
        if !zpaqcfgfile.pcomp.is_empty() {
            info!("emit IR for pcomp");
            write!(output, "pcomp\n{}\n", xcomp_ir_string(true, &pcomp_ir[..])).unwrap();
        }
        write!(output, "end\n").unwrap();
        None
    } else {
        if !hcomp_ir.is_empty() && !optioncfg.suppress_hcomp && zpaqcfgfile.n > 0 {
            info!("generate ZPAQL for hcomp"); // only if a CM model is present and if hcomp is not suppressed
            zpaqcfgfile.hcomp = vec![zpaql::ZPAQLOp::RsetA{n: 255}];
            zpaqcfgfile.hcomp.extend_from_slice(&post_zpaql::replace_array_assignment(gen_zpaql::emit_zpaql(&hcomp_ir, &mut gen_zpaql::Cache{last_hold: HashMap::<zpaql::Loc, ir::IRVar>::new()}, optioncfg), optioncfg));
        }
        if !pcomp_ir.is_empty() && !optioncfg.suppress_pcomp {
            info!("generate ZPAQL for pcomp"); // only if pcomp is not suppressed
            zpaqcfgfile.pcomp = vec![zpaql::ZPAQLOp::RsetA{n: 255}];
            zpaqcfgfile.pcomp.extend_from_slice(&post_zpaql::replace_array_assignment(gen_zpaql::emit_zpaql(&pcomp_ir, &mut gen_zpaql::Cache{last_hold: HashMap::<zpaql::Loc, ir::IRVar>::new()}, optioncfg), optioncfg));
        }
        zpaqcfgfile.finalise(optioncfg).unwrap();
        debug!("hcomp:\n{}", zpaqcfgfile.hcomp.iter().map(|st| format!("  {}", st)).collect::<Vec<String>>()[..].join("\n"));
        debug!("pcomp:\n{}", zpaqcfgfile.pcomp.iter().map(|st| format!("  {}", st)).collect::<Vec<String>>()[..].join("\n"));
        info!("write out ZPAQL cfg file");
        zpaqcfgfile.write_header(&output);
        zpaqcfgfile.write_hcomp(&output, optioncfg);
        zpaqcfgfile.write_pcomp(&output, optioncfg);
        zpaqcfgfile.write_end(&output);
        Some(zpaqcfgfile)
    }
}

/*
#[test]
fn tokenizer() {
    //assert!(grammar::ProgParser::new().parse(tok::tokenize("\n22\n")).is_ok());
    assert_eq!(tok::tokenize("pass"), rtok::tokenize("pass"));  // relies on external programme call
}
*/

