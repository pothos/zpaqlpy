use std::process::{Command, Stdio};
use std::io::{Write, Read};
use std::str::FromStr;

#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    NAME{location: String, value: String},
    NAMEdef{location: String, value: String},
    NAMEbreak{location: String, value: String},
    NAMEcontinue{location: String, value: String},
    NAMEglobal{location: String, value: String},
    NAMEnonlocal{location: String, value: String},
    NAMEwhile{location: String, value: String},
    NAMEif{location: String, value: String},
    NAMEreturn{location: String, value: String},
    NAMEelif{location: String, value: String},
    NAMEelse{location: String, value: String},
    NAMEor{location: String, value: String},
    NAMEnot{location: String, value: String},
    NAMEpass{location: String, value: String},
    NAMEand{location: String, value: String},
    NAMEin{location: String, value: String},
    NAMEis{location: String, value: String},
    NAMENone{location: String, value: String},
    NAMETrue{location: String, value: String},
    NAMEFalse{location: String, value: String},
    COMMENT{location: String, value: String},
    ENCODING{location: String, value: String},
    NL{location: String, value: String},
    LPAR{location: String, value: String},
    RPAR{location: String, value: String},
    LSQB{location: String, value: String},
    RSQB{location: String, value: String},
    COLON{location: String, value: String},
    COMMA{location: String, value: String},
    SEMI{location: String, value: String},
    PLUS{location: String, value: String},
    MINUS{location: String, value: String},
    STAR{location: String, value: String},
    SLASH{location: String, value: String},
    VBAR{location: String, value: String},
    AMPER{location: String, value: String},
    LESS{location: String, value: String},
    GREATER{location: String, value: String},
    EQUAL{location: String, value: String},
    DOT{location: String, value: String},
    PERCENT{location: String, value: String},
    LBRACE{location: String, value: String},
    RBRACE{location: String, value: String},
    EQEQUAL{location: String, value: String},
    NOTEQUAL{location: String, value: String},
    LESSEQUAL{location: String, value: String},
    GREATEREQUAL{location: String, value: String},
    TILDE{location: String, value: String},
    CIRCUMFLEX{location: String, value: String},
    LEFTSHIFT{location: String, value: String},
    RIGHTSHIFT{location: String, value: String},
    DOUBLESTAR{location: String, value: String},
    PLUSEQUAL{location: String, value: String},
    MINEQUAL{location: String, value: String},
    STAREQUAL{location: String, value: String},
    SLASHEQUAL{location: String, value: String},
    PERCENTEQUAL{location: String, value: String},
    AMPEREQUAL{location: String, value: String},
    VBAREQUAL{location: String, value: String},
    CIRCUMFLEXEQUAL{location: String, value: String},
    LEFTSHIFTEQUAL{location: String, value: String},
    RIGHTSHIFTEQUAL{location: String, value: String},
    DOUBLESTAREQUAL{location: String, value: String},
    DOUBLESLASH{location: String, value: String},
    DOUBLESLASHEQUAL{location: String, value: String},
    AT{location: String, value: String},
    ATEQUAL{location: String, value: String},
    ENDMARKER{location: String, value: String},
    NUMBER{location: String, value: u32},
    STRING{location: String, value: String},
    NEWLINE{location: String, value: String},
    INDENT{location: String, value: String},
    DEDENT{location: String, value: String},
    RARROW{location: String, value: String},
    ELLIPSIS{location: String, value: String},
    OP{location: String, value: String},
    AWAIT{location: String, value: String},
    ASYNC{location: String, value: String},
    ERRORTOKEN{location: String, value: String},
}

/* pub fn filter_comments(tokens: Vec<Tok>) -> Vec<Tok> {
    tokens.into_iter().filter(|t| match t {
        &Tok::COMMENT{location: _,value: _} => false,
        &Tok::NL{location: _,value: _} => false ,
        _ => true
    }).collect()
} */

/// (three sections of the template are editable,
///  merge them to two by concatenation of the first with second and first with third)
/// also filter comments
pub fn seperate_sections(tokens: Vec<Tok>) -> (Vec<Tok>, Vec<Tok>) {
    let (mut hcomp, mut pcomp) = (vec![], vec![]);
    let mut in_editable_section = false; let mut section_nr = 0;
    for t in tokens {
        match t {
            Tok::COMMENT{location: _, ref value} => { // @TODO: give them names?
                if !in_editable_section && value.starts_with("### BEGIN OF EDITABLE SECTION") {
                    in_editable_section = true;
                    section_nr += 1;
                } else if in_editable_section && value.starts_with("### END OF EDITABLE SECTION") {
                    in_editable_section = false;
                }
            },
            Tok::NL{location: _,value: _} => {},
            _ => {
                if in_editable_section {
                    match section_nr {
                        1 => { hcomp.push(t.clone()); pcomp.push(t); },
                        2 => { hcomp.push(t); },
                        3 => { pcomp.push(t); },
                        _ => {}, // skip rest as it's the Python standalone runtime code
                    }
                }
            }
        }
    }
    (hcomp, pcomp)
}

/// debug variant which uses the external tokenize module of Python, the Rust tokenizer is in rtok::tokenize
pub fn tokenize(input: &str) -> Vec<Tok> {
    let mut tokens = Vec::new();
    let token_process = Command::new("sh").arg("-c")
        .arg("input_buffer=$(cat; echo x); input_buffer=${input_buffer%x}; printf %s \"$input_buffer\" | python3 -m tokenize -e")
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
        .spawn().unwrap_or_else(|e| { error!("failed to execute process: {}", e); panic!() });
    debug!("started external tokenizer");
    {
        token_process.stdin.unwrap().write_all(input.as_bytes()).unwrap_or_else(|e| panic!("Could not pass input to tokenizer: {}", e) );
    }
    debug!("wrote to external tokenizer");
    let mut output = String::new();
    token_process.stdout.unwrap().read_to_string(&mut output ).unwrap_or_else(|e| panic!("{}", e) );
    let mut err = String::new();
    token_process.stderr.unwrap().read_to_string(&mut err ).unwrap_or_else(|e| panic!("{}", e) );
    if !err.is_empty() {
        error!("external tokenizer python3 -m tokenize -e {} failed:\n{}", input, err);
        panic!();
    }
    debug!("got output of external tokenizer");
    for l in output.lines() {
        let mut l_w = l.split_whitespace();
        let tok_pos = l_w.next().unwrap();
        let tok_name = l_w.next().unwrap();
        let first_apostrophe = l.find("'").unwrap() + 1;
        let mut tok_str = l[first_apostrophe..].to_string();
        let mut last = tok_str.pop().unwrap();
        while last == ' ' {
            last = tok_str.pop().unwrap();
        }
        if last == '"' { last = tok_str.pop().unwrap(); }
        if last != '\'' { tok_str.push(last); }
        let tokn = find_token_for_name(tok_name, &(tok_pos[..tok_pos.len()-1]), &(tok_str.replace("\\\\n", "\n").replace("\\n", "\n")));
        trace!("  {:?},", tokn);
        tokens.push(tokn);
        // println!("{} {} {:?}", tok_pos, tok_name, tok_str);
    }
    debug!("read in tokens");
    tokens
}

pub fn find_token_for_name(name: &str, location: &str, value: &str) -> Tok {
    match name {
        "NAME" => {
            match value {
                "def" => Tok::NAMEdef{location: location.to_string(), value: value.to_string()},
                "break" => Tok::NAMEbreak{location: location.to_string(), value: value.to_string()},
                "continue" => Tok::NAMEcontinue{location: location.to_string(), value: value.to_string()},
                "global" => Tok::NAMEglobal{location: location.to_string(), value: value.to_string()},
                "nonlocal" => Tok::NAMEnonlocal{location: location.to_string(), value: value.to_string()},
                "while" => Tok::NAMEwhile{location: location.to_string(), value: value.to_string()},
                "if" => Tok::NAMEif{location: location.to_string(), value: value.to_string()},
                "return" => Tok::NAMEreturn{location: location.to_string(), value: value.to_string()},
                "elif" => Tok::NAMEelif{location: location.to_string(), value: value.to_string()},
                "else" => Tok::NAMEelse{location: location.to_string(), value: value.to_string()},
                "or" => Tok::NAMEor{location: location.to_string(), value: value.to_string()},
                "not" => Tok::NAMEnot{location: location.to_string(), value: value.to_string()},
                "pass" => Tok::NAMEpass{location: location.to_string(), value: value.to_string()},
                "and" => Tok::NAMEand{location: location.to_string(), value: value.to_string()},
                "in" => Tok::NAMEin{location: location.to_string(), value: value.to_string()},
                "is" => Tok::NAMEis{location: location.to_string(), value: value.to_string()},
                "None" => Tok::NAMENone{location: location.to_string(), value: value.to_string()},
                "True" => Tok::NAMETrue{location: location.to_string(), value: value.to_string()},
                "False" => Tok::NAMEFalse{location: location.to_string(), value: value.to_string()},
                _ => Tok::NAME{location: location.to_string(), value: value.to_string()},
                }
            },
        "COMMENT" => Tok::COMMENT{location: location.to_string(), value: value.to_string()},
        "ENCODING" => Tok::ENCODING{location: location.to_string(), value: value.to_string()},
        "NL" => Tok::NL{location: location.to_string(), value: value.to_string()},
        "LPAR" => Tok::LPAR{location: location.to_string(), value: value.to_string()},
        "RPAR" => Tok::RPAR{location: location.to_string(), value: value.to_string()},
        "LSQB" => Tok::LSQB{location: location.to_string(), value: value.to_string()},
        "RSQB" => Tok::RSQB{location: location.to_string(), value: value.to_string()},
        "COLON" => Tok::COLON{location: location.to_string(), value: value.to_string()},
        "COMMA" => Tok::COMMA{location: location.to_string(), value: value.to_string()},
        "SEMI" => Tok::SEMI{location: location.to_string(), value: value.to_string()},
        "PLUS" => Tok::PLUS{location: location.to_string(), value: value.to_string()},
        "MINUS" => Tok::MINUS{location: location.to_string(), value: value.to_string()},
        "STAR" => Tok::STAR{location: location.to_string(), value: value.to_string()},
        "SLASH" => Tok::SLASH{location: location.to_string(), value: value.to_string()},
        "VBAR" => Tok::VBAR{location: location.to_string(), value: value.to_string()},
        "AMPER" => Tok::AMPER{location: location.to_string(), value: value.to_string()},
        "LESS" => Tok::LESS{location: location.to_string(), value: value.to_string()},
        "GREATER" => Tok::GREATER{location: location.to_string(), value: value.to_string()},
        "EQUAL" => Tok::EQUAL{location: location.to_string(), value: value.to_string()},
        "DOT" => Tok::DOT{location: location.to_string(), value: value.to_string()},
        "PERCENT" => Tok::PERCENT{location: location.to_string(), value: value.to_string()},
        "LBRACE" => Tok::LBRACE{location: location.to_string(), value: value.to_string()},
        "RBRACE" => Tok::RBRACE{location: location.to_string(), value: value.to_string()},
        "EQEQUAL" => Tok::EQEQUAL{location: location.to_string(), value: value.to_string()},
        "NOTEQUAL" => Tok::NOTEQUAL{location: location.to_string(), value: value.to_string()},
        "LESSEQUAL" => Tok::LESSEQUAL{location: location.to_string(), value: value.to_string()},
        "GREATEREQUAL" => Tok::GREATEREQUAL{location: location.to_string(), value: value.to_string()},
        "TILDE" => Tok::TILDE{location: location.to_string(), value: value.to_string()},
        "CIRCUMFLEX" => Tok::CIRCUMFLEX{location: location.to_string(), value: value.to_string()},
        "LEFTSHIFT" => Tok::LEFTSHIFT{location: location.to_string(), value: value.to_string()},
        "RIGHTSHIFT" => Tok::RIGHTSHIFT{location: location.to_string(), value: value.to_string()},
        "DOUBLESTAR" => Tok::DOUBLESTAR{location: location.to_string(), value: value.to_string()},
        "PLUSEQUAL" => Tok::PLUSEQUAL{location: location.to_string(), value: value.to_string()},
        "MINEQUAL" => Tok::MINEQUAL{location: location.to_string(), value: value.to_string()},
        "STAREQUAL" => Tok::STAREQUAL{location: location.to_string(), value: value.to_string()},
        "SLASHEQUAL" => Tok::SLASHEQUAL{location: location.to_string(), value: value.to_string()},
        "PERCENTEQUAL" => Tok::PERCENTEQUAL{location: location.to_string(), value: value.to_string()},
        "AMPEREQUAL" => Tok::AMPEREQUAL{location: location.to_string(), value: value.to_string()},
        "VBAREQUAL" => Tok::VBAREQUAL{location: location.to_string(), value: value.to_string()},
        "CIRCUMFLEXEQUAL" => Tok::CIRCUMFLEXEQUAL{location: location.to_string(), value: value.to_string()},
        "LEFTSHIFTEQUAL" => Tok::LEFTSHIFTEQUAL{location: location.to_string(), value: value.to_string()},
        "RIGHTSHIFTEQUAL" | "RIGHTSHIFTEQUAL'>>='" => Tok::RIGHTSHIFTEQUAL{location: location.to_string(), value: ">>=".to_string()},
        "DOUBLESTAREQUAL" => Tok::DOUBLESTAREQUAL{location: location.to_string(), value: value.to_string()},
        "DOUBLESLASH" => Tok::DOUBLESLASH{location: location.to_string(), value: value.to_string()},
        "DOUBLESLASHEQUAL" | "DOUBLESLASHEQUAL'//='" => Tok::DOUBLESLASHEQUAL{location: location.to_string(), value: "//=".to_string()},
        "AT" => Tok::AT{location: location.to_string(), value: value.to_string()},
        "ATEQUAL" => Tok::ATEQUAL{location: location.to_string(), value: value.to_string()},
        "ENDMARKER" => Tok::ENDMARKER{location: location.to_string(), value: value.to_string()},
        "NUMBER" => Tok::NUMBER{location: location.to_string(), value:
            u32::from_str(value).unwrap_or_else(|e| {
                if value.len() < 3 { error!("Could not parse NUMBER {} {}: {}", value, location, e); panic!() }
                else {
                    match &value[..2] {
                        "0x" => u32::from_str_radix(&value[2..], 16).unwrap(),
                        _ => { error!("Could not parse NUMBER {} {}", value, location); panic!() },
                    }
                }
            }),
        },
        "STRING" => Tok::STRING{location: location.to_string(), value:
            {
            let v = if !value.is_empty() && ( (value.starts_with("\"") && value.ends_with("\"") ) ||
                                      (value.starts_with("'") && value.ends_with("'") ) ) {
                    &value[1..value.len()-1]
                } else { value };
            unescape(v)
            }
        },
        "NEWLINE" => Tok::NEWLINE{location: location.to_string(), value: value.to_string()},
        "INDENT" => Tok::INDENT{location: location.to_string(), value: value.to_string()},
        "DEDENT" => Tok::DEDENT{location: location.to_string(), value: value.to_string()},
        "RARROW" => Tok::RARROW{location: location.to_string(), value: value.to_string()},
        "ELLIPSIS" => Tok::ELLIPSIS{location: location.to_string(), value: value.to_string()},
        "OP" => Tok::OP{location: location.to_string(), value: value.to_string()},
        "AWAIT" => Tok::AWAIT{location: location.to_string(), value: value.to_string()},
        "ASYNC" => Tok::ASYNC{location: location.to_string(), value: value.to_string()},
        "ERRORTOKEN" => Tok::ERRORTOKEN{location: location.to_string(), value: value.to_string()},
        _ => { error!("Token not supported: {} {} {}", name, location, value); panic!() },
    }
}

pub fn unescape(v: &str) -> String {
    // @TODO: more unescaping
    v.to_string().replace("\\n", "\n").replace("\\t", "\t").replace("\\'", "'").replace("\\\"", "\"")
}

