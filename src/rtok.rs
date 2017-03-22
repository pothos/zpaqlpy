use regex::Regex;
use tok;

/// port of tokenize.py and token.py

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
enum TokenType {
    ENDMARKER,
    NAME,
    NUMBER,
    STRING,
    NEWLINE,
    INDENT,
    DEDENT,
    LPAR,
    RPAR,
    LSQB,
    RSQB,
    COLON,
    COMMA,
    SEMI,
    PLUS,
    MINUS,
    STAR,
    SLASH,
    VBAR,
    AMPER,
    LESS,
    GREATER,
    EQUAL,
    DOT,
    PERCENT,
    LBRACE,
    RBRACE,
    EQEQUAL,
    NOTEQUAL,
    LESSEQUAL,
    GREATEREQUAL,
    TILDE,
    CIRCUMFLEX,
    LEFTSHIFT,
    RIGHTSHIFT,
    DOUBLESTAR,
    PLUSEQUAL,
    MINEQUAL,
    STAREQUAL,
    SLASHEQUAL,
    PERCENTEQUAL,
    AMPEREQUAL,
    VBAREQUAL,
    CIRCUMFLEXEQUAL,
    LEFTSHIFTEQUAL,
    RIGHTSHIFTEQUAL,
    DOUBLESTAREQUAL,
    DOUBLESLASH,
    DOUBLESLASHEQUAL,
    AT,
    ATEQUAL,
    RARROW,
    ELLIPSIS,
    OP,
    AWAIT,
    ASYNC,
    ERRORTOKEN,
    // NT_OFFSET,

    COMMENT,
    NL,
    ENCODING,
}

fn exact_type(typ: TokenType, content: &str) -> TokenType {
    use self::TokenType::*;
    match typ {
        OP => {
            match content {
                "(" =>   LPAR,
                ")" =>   RPAR,
                "[" =>   LSQB,
                "]" =>   RSQB,
                ":" =>   COLON,
                "," =>   COMMA,
                ";" =>   SEMI,
                "+" =>   PLUS,
                "-" =>   MINUS,
                "*" =>   STAR,
                "/" =>   SLASH,
                "|" =>   VBAR,
                "&" =>   AMPER,
                "<" =>   LESS,
                ">" =>   GREATER,
                "=" =>   EQUAL,
                "." =>   DOT,
                "%" =>   PERCENT,
                "{" =>   LBRACE,
                "}" =>   RBRACE,
                "==" =>  EQEQUAL,
                "!=" =>  NOTEQUAL,
                "<=" =>  LESSEQUAL,
                ">=" =>  GREATEREQUAL,
                "~" =>   TILDE,
                "^" =>   CIRCUMFLEX,
                "<<" =>  LEFTSHIFT,
                ">>" =>  RIGHTSHIFT,
                "**" =>  DOUBLESTAR,
                "+=" =>  PLUSEQUAL,
                "-=" =>  MINEQUAL,
                "*=" =>  STAREQUAL,
                "/=" =>  SLASHEQUAL,
                "%=" =>  PERCENTEQUAL,
                "&=" =>  AMPEREQUAL,
                "|=" =>  VBAREQUAL,
                "^=" => CIRCUMFLEXEQUAL,
                "<<=" => LEFTSHIFTEQUAL,
                ">>=" => RIGHTSHIFTEQUAL,
                "**=" => DOUBLESTAREQUAL,
                "//" =>  DOUBLESLASH,
                "//=" => DOUBLESLASHEQUAL,
                "@" =>   AT,
                "@=" =>  ATEQUAL,
                _ => typ,
            }
        },
        _ => typ,
    }
}

/// takes TokenType, string, (startline, startcol), (endline, endcol)
fn token_info(typ: TokenType, val: &str, s: (usize, usize), e: (usize, usize)) -> tok::Tok {
    let (g, h) = s;
    let (j, k) = e;
    let loc = format!("{},{}-{},{}", g, h, j, k);
    let tok_name = format!("{:?}", exact_type(typ, &val));
    tok::find_token_for_name(&tok_name, &loc, &val)
}

fn group2(a: &str, b: &str) -> String {
    format!("({}|{})", a, b)
}
fn group3(a: &str, b: &str, c: &str) -> String {
    format!("({}|{}|{})", a, b, c)
}
fn group4(a: &str, b: &str, c: &str, d: &str) -> String {
    format!("({}|{}|{}|{})", a, b, c, d)
}
fn group5(a: &str, b: &str, c: &str, d: &str, e: &str) -> String {
    format!("({}|{}|{}|{}|{})", a, b, c, d, e)
}
fn group8(a: &str, b: &str, c: &str, d: &str, e: &str, f: &str, g: &str, h: &str) -> String {
    format!("({}|{}|{}|{}|{}|{}|{}|{})", a, b, c, d, e, f, g, h)
}

#[derive(Clone, Debug, PartialEq)]
enum SingleDouble {
    Single,
    Single3,
    Double,
    Double3,
}

fn triple_quoted(t: &str) -> bool {
    match t {
        "'''" | "\"\"\"" |
        "r'''" | "r\"\"\"" | "R'''" | "R\"\"\"" |
        "b'''" | "b\"\"\"" | "B'''" | "B\"\"\"" |
        "br'''" | "br\"\"\"" | "Br'''" | "Br\"\"\"" |
        "bR'''" | "bR\"\"\"" | "BR'''" | "BR\"\"\"" |
        "rb'''" | "rb\"\"\"" | "rB'''" | "rB\"\"\"" |
        "Rb'''" | "Rb\"\"\"" | "RB'''" | "RB\"\"\"" |
        "u'''" | "u\"\"\"" | "U'''" | "U\"\"\"" => true,
        _ => false,
    }
}

fn single_quoted(t: &str) -> bool {
    match t {
        "'" | "\"" |
        "r'" | "r\"" | "R'" | "R\"" |
        "b'" | "b\"" | "B'" | "B\"" |
        "br'" | "br\"" | "Br'" | "Br\"" |
        "bR'" | "bR\"" | "BR'" | "BR\"" |
        "rb'" | "rb\"" | "rB'" | "rB\"" |
        "Rb'" | "Rb\"" | "RB'" | "RB\"" |
        "u'" | "u\"" | "U'" | "U\"" => true,
        _ => false,
    }
}

fn endpats(key: &str) -> Option<SingleDouble> {
    use self::SingleDouble::*;
    match key {
        "'" => Some(Single),
        "\"" => Some(Double),
        "'''" => Some(Single3),
        "\"\"\"" => Some(Double3),
        "r'''" => Some(Single3),
        "r\"\"\"" => Some(Double3),
        "b'''" => Some(Single3),
        "b\"\"\"" => Some(Double3),
        "R'''" => Some(Single3),
        "R\"\"\"" => Some(Double3),
        "B'''" => Some(Single3),
        "B\"\"\"" => Some(Double3),
        "br'''" => Some(Single3),
        "br\"\"\"" => Some(Double3),
        "bR'''" => Some(Single3),
        "bR\"\"\"" => Some(Double3),
        "Br'''" => Some(Single3),
        "Br\"\"\"" => Some(Double3),
        "BR'''" => Some(Single3),
        "BR\"\"\"" => Some(Double3),
        "rb'''" => Some(Single3),
        "rb\"\"\"" => Some(Double3),
        "Rb'''" => Some(Single3),
        "Rb\"\"\"" => Some(Double3),
        "rB'''" => Some(Single3),
        "rB\"\"\"" => Some(Double3),
        "RB'''" => Some(Single3),
        "RB\"\"\"" => Some(Double3),
        "u'''" => Some(Single3),
        "u\"\"\"" => Some(Double3),
        "U'''" => Some(Single3),
        "U\"\"\"" => Some(Double3),
        "r" => None, "R" => None, "b" => None, "B" => None,
        "u" => None, "U" => None,
        _ => panic!("not found in endpats"),
        }
}

/// new helper function as Rust's String::lines does not preserve line endings
fn split_lines(content: &str) -> Vec<String> {
    let mut lines = vec![];
    let mut input = content.to_string();
    while input.len() > 0 {
        let rn_start_byte_ind = input.find("\r\n");
        let n_start_byte_ind = input.find("\n");
        match (rn_start_byte_ind, n_start_byte_ind) {
            (None, None) => { lines.push(input.clone()); input = String::new(); },
            (Some(rn), None) => {
                let (line, rest) = if input.len() == rn+2 { (input.clone(), "".to_string()) } else { let (a,b) = input.split_at(rn+2); (a.to_string(), b.to_string() ) };
                input = rest;
                lines.push(line)
            },
            (Some(rn), Some(n)) => {
                if rn > n {  // use n (rn==n would not be possible)
                    let (line, rest) = if input.len() == n+1 { (input.clone(), "".to_string()) } else { let (a,b) = input.split_at(n+1); (a.to_string(), b.to_string() ) };
                    input = rest;
                    lines.push(line)
                } else {
                    let (line, rest) = if input.len() == rn+2 { (input.clone(), "".to_string()) } else { let (a,b) = input.split_at(rn+2); (a.to_string(), b.to_string() ) };
                    input = rest;
                    lines.push(line)
                }
            },
            (None, Some(n)) => {
                let (line, rest) = if input.len() == n+1 { (input.clone(), "".to_string()) } else { let (a,b) = input.split_at(n+1); (a.to_string(), b.to_string() ) };
                input = rest;
                lines.push(line)
            },
        }
    }
    lines
}

#[allow(non_snake_case)]
pub fn tokenize(input: &str) -> Vec<tok::Tok> {
    use self::TokenType::*;
    let Whitespace = r"[ \f\t]*";
    let Comment = r"#[^\r\n]*";
    // let Ignore = format!("{}{}{}", Whitespace, format!("({}{})*", r"\\\r?\n", Whitespace), format!("({})?", Comment));
    let Name = r"\w+";
    let Hexnumber = r"0[xX][0-9a-fA-F]+";
    let Binnumber = r"0[bB][01]+";
    let Octnumber = r"0[oO][0-7]+";
    let Decnumber = r"(?:0+|[1-9][0-9]*)";
    let Intnumber = group4(&Hexnumber, &Binnumber, &Octnumber, &Decnumber);
    let Exponent = r"[eE][-+]?[0-9]+";
    let Pointfloat = format!("{}({})?", group2(r"[0-9]+\.[0-9]*", r"\.[0-9]+"), Exponent);
    let Expfloat = format!("{}{}", r"[0-9]+", Exponent);
    let Floatnumber = group2(&Pointfloat, &Expfloat);
    let Imagnumber = group2(r"[0-9]+[jJ]", &format!("{}{}", Floatnumber, r"[jJ]"));
    let Number = group3(&Imagnumber, &Floatnumber, &Intnumber);
    let StringPrefix = r"(?:[bB][rR]?|[rR][bB]?|[uU])?";
    // Tail end of ' string.
    let Single = r##"^[^'\\]*(?:\\.[^'\\]*)*'"##;
    // Tail end of " string.
    let Double = r##"^[^"\\]*(?:\\.[^"\\]*)*""##;
    // Tail end of ''' string.
    let Single3 = r"^[^'\\]*(?:(?:\\.|'(?:[^'][^']))[^'\\]*)*'''"; // @TODO: py(?!'') != rustr(?:[^'][^'])
    // Tail end of """ string.
    let Double3 = r##"^[^"\\]*(?:(?:\\.|"(?:[^"][^"]))[^"\\]*)*""""##; // @TODO: py(?!'') != rustr(?:[^"][^"])
    let SingleRE = Regex::new(Single).unwrap();
    let DoubleRE = Regex::new(Double).unwrap();
    let Single3RE = Regex::new(Single3).unwrap();
    let Double3RE = Regex::new(Double3).unwrap();
    let Triple = group2(&format!("{}{}", StringPrefix, "'''"), &format!("{}{}", StringPrefix, r##"""""##));
    // Single-line ' or " string.
    // let xString = group2(&format!("{}{}", StringPrefix, r"'[^\n'\\]*(?:\\.[^\n'\\]*)*'"), &format!("{}{}", StringPrefix, r##""[^\n"\\]*(?:\\.[^\n"\\]*)*""## ) );
    // Because of leftmost-then-longest match semantics, be sure to put the
    // longest operators first (e.g., if = came before ==, == would get
    // recognized as two instances of =).
    let Operator = group8(r"\*\*=?", r">>=?", r"<<=?", r"!=",
                 r"//=?", r"->",
                 r"[-+*/%&@|^=<>]=?",  // don't escape -
                 r"~");
    let Bracket = "[]\\[(){}]";  // strange?
    let Special = group3(r"\r?\n", r"\.\.\.", r"[:;.,@]");
    let Funny = group3(&Operator, &Bracket, &Special);
    // let PlainToken = group4(&Number, &Funny, &xString, &Name);
    // let Token = format!("{}{}", Ignore, PlainToken);
    // First (or only) line of ' or " string.
    let ContStr = group2( &format!("{}{}{}", StringPrefix , r"'[^\n'\\]*(?:\\.[^\n'\\]*)*" , group2("'", r"\\\r?\n") ),
                     &format!("{}{}{}", StringPrefix , r##""[^\n"\\]*(?:\\.[^\n"\\]*)*"## , group2("\"", r"\\\r?\n") ) );
    let PseudoExtras = group3(r"\\\r?\n|$", &Comment, &Triple);  // @TODO: py \Z != rust $
    let PseudoToken = format!("^{}{}", Whitespace, group5(&PseudoExtras, &Number, &Funny, &ContStr, &Name) );
    let PseudoTokenRE = Regex::new(PseudoToken.as_str()).unwrap();  // matches only at the beginning

    let isidentifier = Regex::new(r"^[[:alpha:]][[:word:]]*").unwrap();


    let mut tokens = Vec::new();
    // begin of original _tokenize function
    let mut lnum = 0; let mut parenlev = 0; let mut continued = false;
    let numchars = "0123456789";
    let mut contstr = "".to_string(); let mut needcont = false;
    let mut contline: Option<String> = None;
    let mut indents: Vec<usize> = vec![0];

    // 'stashed' and 'async_*' are used for async/await parsing
    let mut stashed: Option<tok::Tok> = None;
    let mut async_def = false;
    let mut async_def_indent = 0;
    let mut async_def_nl = false;

    let content_lines = split_lines(input); // keep line ending
    let mut content_i = 0;
    let mut endprog = Regex::new("").unwrap();
    let mut strstart = (0,0);
    loop {
        let line = if content_i < content_lines.len() {
            content_i += 1;
            content_lines[content_i-1].clone()
        } else {
            String::new()
        };

        lnum += 1;
        let mut pos = 0;
        let max = line.chars().collect::<Vec<char>>().len();
        if contstr.len() > 0 {                            // continued string
            if line == "" {
                error!("TokenError: EOF in multi-line string {:?}: {}", strstart, contstr);
                panic!("error");
            }
            let endmatch = endprog.captures(line.as_str());
            if endmatch.is_some() {
                pos = endmatch.unwrap().get(0).unwrap().as_str().chars().count();
                let end = pos;
                tokens.push(token_info(STRING, &{ let mut s = String::from(contstr); s.push_str( &charstring(& (line.chars().collect::<Vec<char>>()[..end])) ); s }, strstart, (lnum, end)));
                contstr = "".to_string();
                needcont = false;
                contline = None;
            } else if needcont && (if line.chars().count()>1 { line.chars().nth(line.chars().count()-2).unwrap() != '\\' && line.chars().last().unwrap() != '\n' } else { true }) &&
                ( if line.chars().count()>2 { line.chars().nth(line.chars().count()-2).unwrap() != '\r' && line.chars().nth(line.chars().count()-3).unwrap() != '\\' } else { true } ) {
                tokens.push(token_info(ERRORTOKEN, &{ let mut s = String::from(contstr); s.push_str(line.as_str() ); s }, strstart, (lnum, line.chars().count() ) ));
                contstr = "".to_string();
                contline = None;
                continue;
            } else {
                contstr.push_str(line.as_str());
                contline = Some({ let mut s = contline.unwrap(); s.push_str(line.as_str()); s });
                continue;
            }

        } else if parenlev == 0 && !continued {  // new statement
            if line.len() == 0 {
                break;
            }
            let mut column = 0;
            while pos < max {                   // measure leading whitespace
                if line.chars().nth(pos).unwrap() == ' ' {
                    column += 1;
                } else if line.chars().nth(pos).unwrap() == '\t' {
                    column = (column/8 + 1)*8;  // tabsize=8
                } else if line.chars().nth(pos).unwrap() == '\x0C' {  // \f
                    column = 0;
                } else {
                    break;
                }
                pos += 1;
            }
            if pos == max {
                break;
            }

            if "#\r\n".contains(line.chars().nth(pos).unwrap()) {           // skip comments or blank lines
                if line.chars().nth(pos).unwrap() == '#' {
                    let cs = charstring(&(line.chars().collect::<Vec<char>>()[pos..]));
                    let comment_token = cs.trim_right();  // only remove right "\r\n" not all whitespace?
                    let nl_pos = pos + comment_token.chars().count();
                    tokens.push(token_info(COMMENT, comment_token, (lnum, pos), (lnum, pos + comment_token.chars().count())));
                    tokens.push(token_info(NL, &charstring(&(line.chars().collect::<Vec<char>>()[nl_pos..])), (lnum, nl_pos), (lnum, line.chars().count() )));
                } else {
                    let tp = if line.chars().nth(pos).unwrap() == '#' { COMMENT } else { NL } ;
                    tokens.push(token_info(tp, &charstring(&(line.chars().collect::<Vec<char>>()[pos..])), (lnum, pos), (lnum, line.chars().count())));
                }
                continue;
            }
            if column > *(indents.last().unwrap()) {           // count indents or dedents
                indents.push(column);
                tokens.push(token_info(INDENT, &charstring(&(line.chars().collect::<Vec<char>>()[..pos])), (lnum, 0), (lnum, pos)));
            }
            while column < *(indents.last().unwrap()) {
                if !indents.contains(&column) {
                    error!("IndentationError: unindent does not match any outer indentation level {:?}", ("<tokenize>", lnum, pos, line));
                    panic!("error");
                }
                indents.pop().unwrap_or(0);

                if async_def && async_def_indent >= *(indents.last().unwrap()) {
                    async_def = false;
                    async_def_nl = false;
                    async_def_indent = 0;
                }

                tokens.push(token_info(DEDENT, "", (lnum, pos), (lnum, pos)));
            }
            if async_def && async_def_nl && async_def_indent >= *(indents.last().unwrap()) {
                async_def = false;
                async_def_nl = false;
                async_def_indent = 0;
            }

        } else {                                  // continued statement
            /*if line.len() == 0 && !continued {
                break;  // only needed for debugging if code stops working, see if parenlev is still -1 and not 0
            }*/
            if line.len() == 0 {
                error!("TokenError: (2) EOF in multi-line statement {:?},{},{},{:?},{}.", (lnum, 0), pos, max, continued, parenlev);
                panic!("error");
            }
            continued = false;
        }

        while pos < max {
            let tm = charstring(&(line.chars().collect::<Vec<char>>()[pos..]) );
            let pseudomatch = PseudoTokenRE.captures(&tm);
            if pseudomatch.is_some() {                                // scan for tokens
                let pv = pseudomatch.unwrap();
                /*if tm.contains("relevant") {
                    println!("{}", tm);
                    println!("{:?}", pv);
                }*/
                let first = pv.get(1).unwrap().as_str();
                let matchstart = if tm.starts_with(first) {0} else { tm.split(first).next().unwrap().chars().count() };
                let matchlen = first.chars().count();
                let (start, end) = (pos+matchstart, pos+matchstart+matchlen);
                pos = end;
                let (spos, epos) = ((lnum, start), (lnum, end));
                if start == end {
                    continue;
                }
                let (mut token, initial) = ( charstring(&(line.chars().collect::<Vec<char>>()[start..end]) ), line.chars().nth(start).unwrap() );
                /*if tm.contains("relevant") {
                    println!("{}:{}", initial, token);
                }*/
                // println!("{}-{}:{}:{}", start, end, token, line);
                // ordinary number
                if numchars.contains(initial) ||
                    (initial == '.' && token != "." && token != "...") {
                        tokens.push(token_info(NUMBER, &token, spos, epos));
                } else if initial == '\r' || initial == '\n' {
                    if stashed.is_some() {
                        tokens.push(stashed.take().unwrap());
                    }
                    if parenlev > 0 {
                        tokens.push(token_info(NL, &token, spos, epos));
                    } else {
                        tokens.push(token_info(NEWLINE, &token, spos, epos));
                        if async_def {
                            async_def_nl = true;
                        }
                    }
                } else if initial == '#' {
                    assert!(!token.ends_with("\n"));
                    if stashed.is_some() {
                        tokens.push(stashed.take().unwrap());
                    }
                    tokens.push(token_info(COMMENT, &token, spos, epos));
                } else if triple_quoted(&token) {
                    endprog = match endpats(&token) {
                                    Some(SingleDouble::Single3) => Single3RE.clone(),
                                    Some(SingleDouble::Single) => SingleRE.clone(),
                                    Some(SingleDouble::Double) => DoubleRE.clone(),
                                    Some(SingleDouble::Double3) => Double3RE.clone(),
                                    _ => panic!("should not be reached?"),
                    };
                    let tx = charstring(&(line.chars().collect::<Vec<char>>()[pos..]));
                    let endmatch = endprog.captures( &tx );
                    if endmatch.is_some() {                           // all on one line
                        let ev = endmatch.unwrap();
                        // println!("{:?},{:?}", endprog, ev);
                        // @TODO: original group was 1 for: ^[^"\\]*(?:(?:\\.|"(?:[^"][^"]))[^"\\]*)*""", here set to 0 because grouping seems to be different in Rust to Python
                        pos = pos + ev.get(0).unwrap().as_str().chars().count(); // unwrap_or_else: || panic!("{},{},{}", line, pos, tx)
                        token = charstring(&(line.chars().collect::<Vec<char>>()[start..pos]));
                        tokens.push(token_info(STRING, &token, spos, (lnum, pos)));
                    } else {
                        strstart = (lnum, start);           // multiple lines
                        contstr = charstring(&(line.chars().collect::<Vec<char>>()[start..]));
                        contline = Some(line.clone());
                        break;
                    }
                } else if single_quoted(&{let mut x = String::new(); x.push(initial); x }) ||
                    single_quoted(&charstring(&(token.chars().collect::<Vec<char>>()[..( match token.chars().count() {0 => 0, 1 => 1, _ => 2} ) ])) ) ||
                    single_quoted(&charstring(&(token.chars().collect::<Vec<char>>()[..( match token.chars().count() {0 => 0, 1 => 1, 2 => 2, _ => 3} )])) ) {
                    if token.chars().last().unwrap() == '\n' {                  // continued string
                        strstart = (lnum, start);
                        endprog = match endpats(&token) {
                                    Some(SingleDouble::Single3) => Single3RE.clone(),
                                    Some(SingleDouble::Single) => SingleRE.clone(),
                                    Some(SingleDouble::Double) => DoubleRE.clone(),
                                    Some(SingleDouble::Double3) => Double3RE.clone(),
                                    _ => {
                                        match endpats(&{let mut x = String::new(); x.push(token.chars().nth(1).unwrap() ); x }) {
                                                    Some(SingleDouble::Single3) => Single3RE.clone(),
                                                    Some(SingleDouble::Single) => SingleRE.clone(),
                                                    Some(SingleDouble::Double) => DoubleRE.clone(),
                                                    Some(SingleDouble::Double3) => Double3RE.clone(),
                                                    _ => {
                                                        match endpats(&{let mut x = String::new(); x.push(token.chars().nth(2).unwrap() ); x }) {
                                                                    Some(SingleDouble::Single3) => Single3RE.clone(),
                                                                    Some(SingleDouble::Single) => SingleRE.clone(),
                                                                    Some(SingleDouble::Double) => DoubleRE.clone(),
                                                                    Some(SingleDouble::Double3) => Double3RE.clone(),
                                                                    _ => {
                                                                        panic!("should not happen, is it valid Python?")
                                                                    },
                                                        }
                                                    },
                                        }
                                    },
                        };
                        contstr = charstring(&(line.chars().collect::<Vec<char>>()[start..]));
                        needcont = false;
                        contline = Some(line.clone());
                        break;
                    } else {                                  // ordinary string
                        tokens.push(token_info(STRING, &token, spos, epos));
                    }
                } else if isidentifier.is_match(&{let mut x = String::new(); x.push(initial); x }) {    // ordinary name
                    if token == "async" || token == "await" {
                        if async_def {
                            tokens.push(token_info(if token == "async" {ASYNC} else {AWAIT} , &token, spos, epos));
                            continue;
                        }
                    }

                    let tok = token_info(NAME , &token, spos, epos);
                    if token == "async" && !stashed.is_some() {
                        stashed = Some(tok);
                        continue;
                    }

                    if token == "def" {
                        if stashed.is_some() && is_name(stashed.as_ref().unwrap())
                                && is_async(stashed.as_ref().unwrap()) {
                            async_def = true;
                            async_def_indent = *(indents.last().unwrap());
                            token_info(ASYNC , "async", (0,0), (0,0));
                            stashed = None;
                            }
                    }
                    if stashed.is_some() {
                        tokens.push(stashed.take().unwrap());
                    }
                    tokens.push(tok);
                } else if initial == '\\' {         // continued stmt
                    continued = true;
                } else {
                    if "([{".contains(initial) {
                        parenlev += 1;
                    } else if ")]}".contains(initial) {
                        parenlev -= 1;
                    }
                    if stashed.is_some() {
                        tokens.push(stashed.take().unwrap());
                    }
                    tokens.push(token_info(OP, &token, spos, epos));
                }
            } else {
                tokens.push(token_info(ERRORTOKEN, &{let mut x = String::new(); x.push(line.chars().nth(pos).unwrap()); x }, (lnum, pos), (lnum, pos+1) ));
                pos += 1;
            }
        }
    }
    if stashed.is_some() {
        tokens.push(stashed.take().unwrap());
    }
    if indents.len() > 0 {
        indents.remove(0);
    }
    for _ in indents {                 // pop remaining indent levels
        tokens.push(token_info(DEDENT, "", (lnum, 0), (lnum, 0)));
    }
    tokens.push(token_info(ENDMARKER, "", (lnum, 0), (lnum, 0)));
    tokens
}

// convert char list to a new, owned string
pub fn charstring(chars: &[char]) -> String {
    let mut s = String::new();
    for c in chars {
        s.push(*c);
    }
    s
}

fn is_async(tok: &tok::Tok) -> bool {
    use tok::Tok::*;
    match tok {
        &NAME{location: _, ref value} => { value == "async" },
        _ => false,
    }
}

fn is_name(tok: &tok::Tok) -> bool {
    use tok::Tok::*;
    match tok {
        &NAME{location: _, value: _} |
        &NAMEdef{location: _, value: _} |
        &NAMEbreak{location: _, value: _} |
        &NAMEcontinue{location: _, value: _} |
        &NAMEglobal{location: _, value: _} |
        &NAMEnonlocal{location: _, value: _} |
        &NAMEwhile{location: _, value: _} |
        &NAMEif{location: _, value: _} |
        &NAMEreturn{location: _, value: _} |
        &NAMEelif{location: _, value: _} |
        &NAMEelse{location: _, value: _} |
        &NAMEor{location: _, value: _} |
        &NAMEnot{location: _, value: _} |
        &NAMEpass{location: _, value: _} |
        &NAMEand{location: _, value: _} |
        &NAMEin{location: _, value: _} |
        &NAMEis{location: _, value: _} |
        &NAMENone{location: _, value: _} |
        &NAMETrue{location: _, value: _} |
        &NAMEFalse{location: _, value: _} => true,
        _ => false,
    }
}

