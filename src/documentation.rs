
pub static INFO_ZPAQ: &'static str = "
The ZPAQ format and the zpaq archiver
=====================================

*** The ZPAQ Open Standard Format for Highly Compressed Data ***

Based on the idea to deliver the decompression algorithm together with
the compressed data this archive format wants to solve the problem that
changes to the algorithm need new software at the recipient's device.
Also it acknowledges the fact that different input data should be
handled with different compression techniques.
The PAQ compression programmes typically use context mixing i.e.
mixing different predictors which are context-aware for usage in an
arithmetic encoder, and thus often achieve the best known compression
results. The ZPAQ archiver is the successor to them and also supports
more simple models like LZ77 and BWT depending on the input data.
It is only specified how decompression takes place. The format makes
use of predefined context model components which can be woven into
a network, a binary code for context computation for components and a
postprocessor which reverts a transformation on the input data that
took place before the data was passed to the context mixing and
encoding phase. The postprocessor is also delivered as a bytecode
like the context computation code before the compressed data begins.
  Specification: http://mattmahoney.net/dc/zpaq206.pdf

*** zpaq - Incremental Journaling Backup Utility and Archiver ***

The end user archiver supports incremental backups with deduplication as
well as flat streaming archives (ZPAQ format Level 1). It picks simple
or more complex depending on whether they perform for the input data
and which compression level was specified for the files to append
to the archive. Arbitrary algorithms are not supported, but a good
variety of specialised and universal methods is available.
  Homepage: http://mattmahoney.net/dc/zpaq.html
  Working principle: http://mattmahoney.net/dc/zpaq_compression.pdf

*** zpaqd - development tool for new algorithms ***

The zpaqd development tool only allows creation of streaming mode
archives, but therefore accepts a ZPAQ configuration file containing
information on the used context mixing components, the ZPAQL programme
for context computation and the ZPAQL postprocessing programme in order
to revert a possible transformation that took place (LZ77, BWT,
E8E9 for x86 files or any custom transformation), which is applied
before compression an externally called programme named in the
configuration. There are special configurations for JPG, BMP and more.
  Homepage: http://mattmahoney.net/dc/zpaqutil.html
";

pub static INFO_ZPAQL: &'static str = "
The ZPAQL virtual machine and bytecode
=======================================

The virtual machine consists of the 32-bit registers A, B, C and D,
an 1-bit condition flag F, 256 32-bit registers R0…R255 and the
arrays M (8-bit elements) and H (32-bit elements).

The comp section specifies the size of the arrays and which
context-mixing components are used. Each component i gets it's context
input from the entry in H[i] after each run of the hcomp bytecode,
which is called for each input byte of the preprocessed data, which
either is to be stored through arithmetic coding in compression phase
or is retrieved through decoding in decompression phase with optional
postprocessing done by the pcomp bytecode. The comp section is written
to the archive in byte representation as well as the bytecodes of
hcomp and pcomp (if they are used). As the preprocessor might be any
external programme or also included in the compressing archiver and is
of no use for decompression it is therefore not mentioned in the
archive anymore. The opcodes of the bytecode are defined in the
ZPAQ specification: http://mattmahoney.net/dc/zpaq206.pdf
Also there you will find information about the syntax for context
models and their meaning. You might also take a look at the
reference decoder unzpaq206.cpp and specially libzpaq.h which is
used by both zpaq and zpaqd.

There is an assembly language for the bytecode in the source file of
a ZPAQ configuration for usage with zpaqd. It not only contains the
opcodes but also helper instructions like if, ifnot, do, while, … and
their long-distance-jump versions ifl, … which will be converted to
conditional jumps. White space can only occur between opcode bytes.
So 'a=b' is a 1-byte opcode and can't be written as 'a = b' and
'a=r 255' is a 2-byte opcode which therefore contains whitespace.
Comments are written in brackets.

Example mfast.cfg without a pcomp section:
comp 2 2 0 0 4 (hh hm ph pm n)
                    (where H gets the size of 2^hh in hcomp or 2^ph in comp,
                     M 2^hm or 2^pm and n is the number of
                     context-mixing components)
  0 cm 19 4   (will get an order 1 context)
  1 icm 16    (order 2, chained to isse)
  2 isse 19 1 (order 4, has reference to ICM component 1)
  3 mix2 0 0 2 24 0 (moderate adapting mixer between CM and ISSE
                     based on which predicts better, no contexts even for bits)
  (ICM and ISSE part adapted from fast.cfg)
hcomp
  r=a 2 (R2 = A, input byte in R2)
  d=0
  a<<= 9 *d=a (H[D] = A) (set context to actual byte)
  (leaving first 9 bits free for the partially decoded byte)
  a=r 2 (A = R2)
  *b=a (M[B] = A) (save input byte in rotating buffer)
                  (full M is used with pointer b)
  a=0 hash (shortcut for A = (A + M[B] + 512) * 773)
  b-- hash
  d= 1 *d=a (order 2 hash for H[1])
  b-- hash b-- hash
  d= 2 *d=a (order 4 hash for H[2])
  (H[3] stays 0 as fixed context for MIX2)
  halt (execution stops here for this input byte)
end

For more examples see: http://mattmahoney.net/dc/zpaqutil.html
";

pub static INFO_ZPAQLIR: &'static str = "
The zpaqlir intermediate representation language
================================================

The IR was chosen to be close to ZPAQL but easier to write.
While ZPAQL has the registers A, B, C, D, F, R0…255 and arrays H and M
the IR gives only control about R, H and M and no other registers.
There are no new data structures added. So the other registers can be used for
address computation and temporary calculations when the IR is converted to
ZPAQL. So the temporary variables t0…t255 of the IR are a direct mapping to
registers Ri.
Because the input byte is in A at the beginning of hcomp/pcomp execution the IR
relies on the guarantee that R255 = A before the first instruction.

*** Grammar ***

stmt -> var ”=” var (op var)?
        var ”=” uop var
        ”if” var ”goto” label
        ”ifN” var ”goto” label
        ”ifEq” var var ”goto” label (to be used for optimizations)
        ”ifNeq” var var ”goto” label
        ”goto” label
        ”:” label ”:”
        ”halt”
        ”error”
        ”out” var
var ->  t | ”H[” t ”]” | ”H[t0+” x ”]” | ”H[t252+” x ”]” | ”H[” x ”]”
        ”M[” t ”]” | ”M[” x ”]” | x
op -> ”+” | ”-” | ”*” | ”/” | ”//” | ”%” | ”**” | ”<<” | ”>>” | ”|” | ”^”
      ”&” | ”or” | ”and” | ”==” | ”!=” | ”<” | ”<=” | ”>” | ”>=”
uop -> ”!” | ”~” | ”-”
t -> ”t0” | … | ”t255”
x -> ”0” | … | ”4294967295”
label -> [a-z_0-9~A-Z]+
comment -> ”#…\\n”

*** Notes ***

The var on the left side of an assignment can not be a number.
The operators or and and differ from the binary versions | and & as they
represent the semantics of Python or and and i.e. they evaluate to the original
value and not simply to the boolean choices of 1 and 0 for true and false while
the binary operators to a bitwise AND and OR. Operator !v tests against v==0
while ~v inverts the bits.

The local variables of a function are held on the stack which is produced by
expanding H beyond its defined size for the configuration. Global variables are
held in the beginning of the stack. The temporary variables t0…t251 are used for
intermediate or address computations whereas t0 is the base pointer for the
stack and t252 is a copy of the global base pointer. t255 holds the last input
byte, t254 the reading state and t253 the read byte for the API function
read_b() which stops the execution and returns to the caller with the newly
acquired byte when the bytecode is run again.

The temporary variables have to be saved on the stack before a call and also the
current base pointer and then the return ID for the jump table and need to be
saved there as part of the calling convention. The new base pointer in t0 points
at the return ID. Arguments passed come afterwards and the called function will
address them via H[t0+x] . On return the previous base pointer will be restored
to t0 and the return ID is copied in t2 for the jumper table while the return
value is in t1 before the jump to the code for the jump table is done in order
to return after the call instruction.

";
// @TODO: write documentation: minimal example of a hcomp+pcomp lz1 IR(!) port when IR is accepted as input.ir


pub static INFO_ZPAQLPY: &'static str = "
The zpaqlpy Python-subset
=========================

*** Grammar ***

For user-defined sections of the template. Not all is supported but anyway
included for specific error messages instead of parser errors (e.g. nonlocal,
dicts, strings or the @-operator for matrix multiplication).
Listed here are productions with NUMBER, NAME, ”symbols”, NEWLINE, INDENT,
DEDENT or STRING as terminals, nonterminals are defined on the left side of ->.

    Prog -> (NEWLINE* stmt)* ENDMARKER?
    funcdef -> ”def” NAME Parameters ”:” suite
    Parameters -> ”(” Typedargslist? ”)”
    Typedargslist -> Tfpdef (”=” test)? (”,” Tfpdef (”=” test)?)* (”,” (”**” Tfpdef)?)?
    Tfpdef -> NAME (”:” test)?
    stmt -> simple_stmt | compound_stmt
    simple_stmt -> small_stmt (”;” small_stmt)* ”;”? NEWLINE
    small_stmt -> expr_stmt, pass_stmt, flow_stmt, global_stmt, nonlocal_stmt
    expr_stmt -> (store_assign augassign test) | ((store_assign ”=”)? test)
    store_assign -> NAME (”[” test ”]”)?
    augassign -> ”+=” | ”-=” | ”*=” | ”@=” | ”//=” | ”/=” | ”%=” | ”&=” | ”|=” | ”^=” | ”<<=” | ”>>=” | ”**=”
    pass_stmt -> ”pass”
    flow_stmt -> break_stmt | continue_stmt | return_stmt
    break_stmt -> ”break”
    continue_stmt -> ”continue”
    return_stmt -> ”return” test
    global_stmt -> ”global” NAME (”,” NAME)*
    nonlocal_stmt -> ”nonlocal” NAME (”,” NAME)*
    compound_stmt -> if_stmt | while_stmt | funcdef
    if_stmt -> ”if” test ”:” suite (”elif” test ”:” suite)* (”else” ”:” suite)?
    while_stmt -> ”while” test ”:” suite (”else” ”:” suite)?
    suite -> simple_stmt, NEWLINE INDENT stmt+ DEDENT
    test -> or_test
    test_nocond -> or_test
    or_test -> and_test (”or” and_test)*
    and_test -> not_test (”and” not_test)*
    not_test -> comparison | (”not” not_test)
    comparison -> expr (comp_op expr)*
    comp_op -> ”<” | ”>” | ”==” | ”>=” | ”<=” | ”!=” | ”in” | ”not” ”in” | ”is” | ”is” ”not”
    expr -> xor_expr (”|” xor_expr)*
    xor_expr -> and_expr (”^” and_expr)*
    and_expr -> shift_expr (”&” shift_expr)*
    shift_expr -> arith_expr | (arith_expr (shift_op arith_expr)+)
    shift_op -> ”<<” | ”>>”
    arith_expr -> term | (term (t_op term)+)
    t_op -> ”+” | ”-”
    term -> factor (f_op factor)*
    f_op -> ”*” | ”@” | ”/” | ”%” | ”//”
    factor -> (”+” factor) | (”-” factor) | (”~” factor) | power
    power -> atom_expr (”**” factor)?
    atom_expr -> (NAME ”(” arglist? ”)”) | (NAME ”[” test ”]”) | atom
    atom -> (”(” test ”)”) | (”” dictorsetmaker? ””) | NUMBER | STRING+ | ”...”
            ”None” | ”True” | ”False” | NAME
    dictorsetmaker -> dictorsetmaker_t (”,” dictorsetmaker_t)* ”,”?
    dictorsetmaker_t -> test ”:” test
    arglist -> test (”,” test)* ”,”?

*** Notes ***

An input has to be organised like the template, so best is to fill it out with
the values for hh, hm, ph, pm like in a ZPAQ configuration to define the size of
H and M in hcomp and pcomp sections. In the dict which serves for calculation of
n (i.e. number of context mixing components) you have to specify the components
as in a ZPAQ configuration file, arguments are documented in the specification
(see --info-zpaq for link).
Only valid Python programmes without exceptions are supported as input, so run
them standalone before compiling.
For the arrays on top of H or M there is no boundary check, please make sure
the Python version works correct. If you need a ringbuffer on H or M, you have
to use % len(hH) or &((1<<hh)-1) and can not rely on integer overflows or the
modulo-array-length operation on indices in H or M like in plain ZPAQL because
H is expanded to contain the stack (and also due to the lack of overflows when
running the plain Python script)
Only positive 32-bit integers can be used, no strings, lists, arbitrary big
numbers, classes, closures and (function) objects.

*** Input File ***

Must be a runnable Python 3.5 file in form of the template and encoded as UTF-8
without a BOM (Byte-Order-Mark). The definitions at the beginning should be
altered and own code inserted only behind. The other two editable sections can
refer to definitions in the first section.

        Template Sections (--emit-template > source.py)         |   Editable?
================================================================|==============
  Definition of the ZPAQ configuration header data (memory size,|
  context mixing components) and optionally functions and       |
  variables used by both hcomp and pcomp                        |      yes
________________________________________________________________|______________
  API functions for input and output, initialization of memory  |       no
________________________________________________________________|______________
  function hcomp and associated global variables and functions  |      yes
________________________________________________________________|______________
  function pcomp and associated global variables and functions  |      yes
________________________________________________________________|______________
  code for standalone execution of the Python file analog to    |
  running a ZPAQL configuration with zpaqd r [cfg] p|h          |       no

*** Exposed API ***

The 32- or 8-bit memory areas H and M are available as arrays hH, pH, hM, pM
depending on being a hcomp or pcomp section with size 2**hh , 2**hm , 2**ph ,
2**pm defined in the header as available constants hh, hm, ph, pm.
There is support for len(hH), len(pH), len(hM), len(pM) instead of calculating
2**hh. But in general len() is not supported, see len_hH() below for dynamic
arrays. NONE is a shortcut for 0 - 1 = 4294967295.

      Other functions       |                   Description
============================|==================================================
c = read_b()                | Read one input byte, might leave VM execution and
                            | return to get next
____________________________|__________________________________________________
push_b(c)                   | Put read byte c back, overwrites if already
                            | present (no buffer)
____________________________|__________________________________________________
c = peek_b()                | Read but do not consume next byte, might leave VM
                            | execution and return to get next
____________________________|__________________________________________________
out(c)                      | In pcomp: write c to output stream
____________________________|__________________________________________________
error()                     | Execution fails with ”Bad ZPAQL opcode”
____________________________|__________________________________________________
aref = alloc_pH(asize), …   | Allocate an array of size asize on pH/pM/hH/hM
____________________________|__________________________________________________
aref = array_pH(intaddr), … | Cast an integer address back to a reference
____________________________|__________________________________________________
len_pH(aref), …             | Get the length of an array in pH/pM/hH/hM
____________________________|__________________________________________________
free_pH(aref), …            | Free the memory in pH/pM/hH/hM again by
                            | destructing the array

If backend implementations addr_alloc_pH(size), addr_free_pH(addr), … are
defined then dynamic memory management is available though the API functions
alloc_pM and free_pM. The cast array_pH(numbervar) can be used to save a type
check in ZPAQL at runtime. Also in plain Python the cast from an address is
needed after an array reference was itself stored into H and thus became an
address number and is then retrieved as a number again instead of a reference.
In general there are no boxed types but by context a variable is used as
address.

The template provides sample implementations of addr_alloc_pM, addr_free_pM , ….
The returned pointer is expected to point at the first element of the array. One
entry before the first element is used to store whether this memory section is
free or not. Before that the length of the array is store, i.e.
H[arraypointer-2] for arrays in H and the four bytes
M[arraypointer-5]…M[arraypointer-2] of the 32-bit length for arrays in M.
The last addressable starting point for any list is 2147483647 == (1<<31) - 1
because the compiler uses the 32nd bit to distinguish between pointers to M/H.

Beside these constraints the implementations are free how to find a free region.
The example uses getter and setter functions to for the 32-bit length value as
four bytes in M. For allocation it skips over the blocks from the beginning
until a sufficiently sized block is found. If this block is bigger then the rest
of it is kept free and might be merged with the next block if it is also free.
That also happens when a block is freed again, then it is even merged with the
previous block if that is free.

";

pub static INFO_TUTORIAL: &'static str = "
Tutorial: Writing new code
==========================

A context mixing model with a preprocessor for run length encoding.
Three components are used to form the network.

Create a new template which will then be modified at the beginning and the pcomp/hcomp sections:
  ./zpaqlpy --emit-template > rle_model.py
  chmod +x rle_model.py

First the size of the arrays H and M for each section, hcomp and pcomp needs to be specified:
  hh = 2  # i.e. size is 2**2 = 4, because H[0], H[1], H[2] are the inputs for the components

The first component should give predictions based on the byte value and the second component based on the run length,
both give predictions for the next count and the next value.
Then the context-mixing components are combined to a network:

  n = len({
    0: \"cm 19 22\",  # context table size 2*19 with partly decoded byte as 9 bit hash xored with the context, count limit 22
    1: \"cm 19 22\",
    2: \"mix2 1 0 1 30 0\",  # will mix 0 and 1 together, context table size 2**1 with and-0 masking of the partly decoded byte which is added to the context, learning rate 30
  })

Each component i gets its context input from the entry in H[i] after each run of
the hcomp function, which is called for each input byte of the preprocessed data,
which either is to be stored through arithmetic coding in compression phase
or is retrieved through decoding in decompression phase with following
postprocessing done by calls of the pcomp function.

Then we specify a preprocessor:
  pcomp_invocation = './simple_rle'

The context-mixing network is written to the archive in byte representation
as well as the bytecode for hcomp and pcomp (if they are used).
The preprocessor command is needed when the compiled file is used with zpaqd
if a pcomp section is present.
As the preprocessor might be any external programme or also included in the
compressing archiver and is of no use for decompression it is therefore not
mentioned in the archive anymore.

Create the preprocessor file and fill it:
$ chmod +x simple_rle
$ cat ./simple_rle
#!/usr/bin/env python3
import sys
input = sys.argv[1]
output = sys.argv[2]
with open(input, mode='rb') as fi:
  with open(output, mode='wb') as fo:
      last = None
      count = 0
      data = []
      for a in fi.read():
        if a != last or count == 255:  # count only up to 255 to use one byte
          if last != None:  # write out the pair
            data.append(last)
            data.append(count)
          last = a  # start counting
          count = 1
        else:
          count += 1  # continue counting
      if last != None:
        data.append(last)
        data.append(count)
      fo.write(bytes(data))

Then we need code in the pcomp section to undo this transform:

case_loading = False
last = NONE

def pcomp(c):
  global case_loading, last
  if c == NONE:  # start of new segment, so restart our code
    case_loading = False
    last = NONE
    return
  if not case_loading:  # c is byte to load
    case_loading = True
    last = c
  else:  # write out content of last c times
    case_loading = False
    while c > 0:
      c-= 1
      out(last)

So now it should produce the same file as the input file:
  ./simple_rle INPUTFILE input.rle
  ./rle_model.py pcomp input.rle input.norle
  cmp INPUTFILE input.norle

And we can already try it, even if hcomp does not compute the context data yet (so compression is not really good):
  ./zpaqlpy rle_model.py
  ./zpaqd c rle_model.cfg archive.zpaq FILE FILE FILE

Now we can add hcomp code to improve compression by adaptive prediction:

at_counter = False  # if false, then c is byte, otherwise c is a counter
last_value = NONE
last_counter = NONE

def hcomp(c):  # pcomp bytecode is passed first (or 0 if there is none)
  global at_counter, last_value, last_counter
  if at_counter:
    last_counter = c
  else:
    last_value = c
  # first part of the context for the first CM is the byte repilcated and
  # the second part is whether we are at a counter (then we predict for a byte) or vice versa
  hH[0] = last_value << 1 + at_counter  # at_counter occupies one bit, therefore shift
  hH[0] <<= 9  # again shift to the side because of the xor with the partially decoded byte
  # second CM same but uses the counter for prediction
  hH[1] = last_counter << 1 + at_counter
  hH[1] <<= 9
  hH[2] = at_counter + 0  # context for mixer: is at counter (1) or not (0)
  at_counter = not at_counter

We need to compile again before we run the final ZPAQ configuration file:
  ./zpaqlpy rle_model.py
  ./zpaqd c rle_model.cfg archive.zpaq FILE FILE FILE

zpaqd needs to have simple_rle in the same folder because we specified pcomp_invocation = \"./simple_rle\"

";

