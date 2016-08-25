zpaqlpy compiler
================

Compiles a zpaqlpy source file (a Python-subset) to a ZPAQ configuration file for usage with zpaqd.

That way it is easy to develop new compression algorithms with ZPAQ.

Or to bring a decompression algorithm to the ZPAQ format so that the compressed data can be stored in a ZPAQ archive without breaking compatibility.

The Python source files are standalone executable with Python 3 (tested: 3.4, 3.5).

Jump to the end for a tutorial or look into test/lz1.py, test/pnm.py or test/brotli.py for an example.

Build with: `make zpaqlpy`
To build again: `make clean`

Copyright (C) 2016 Kai Lüke kailueke@riseup.net

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <http://www.gnu.org/licenses/>.


The ZPAQ format and the zpaq archiver
=====================================

**The ZPAQ Open Standard Format for Highly Compressed Data**

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

**zpaq - Incremental Journaling Backup Utility and Archiver**

The end user archiver supports incremental backups with deduplication as
well as flat streaming archives (ZPAQ format Level 1). It picks simple
or more complex depending on whether they perform for the input data
and which compression level was specified for the files to append
to the archive. Arbitrary algorithms are not supported, but a good
variety of specialised and universal methods is available.

Homepage: http://mattmahoney.net/dc/zpaq.html

Working principle: http://mattmahoney.net/dc/zpaq_compression.pdf

**zpaqd - development tool for new algorithms**

The zpaqd development tool only allows creation of streaming mode
archives, but therefore accepts a ZPAQ configuration file containing
information on the used context mixing components, the ZPAQL programme
for context computation and the ZPAQL postprocessing programme in order
to revert a possible transformation that took place (LZ77, BWT,
E8E9 for x86 files or any custom transformation), which is applied
before compression an externally called programme named in the
configuration. There are special configurations for JPG, BMP and more.

Homepage: http://mattmahoney.net/dc/zpaqutil.html

The zpaqlpy Python-subset
=========================

**Grammar**

For user-defined sections of the template. Not all is supported but anyway
included for specific error messages instead of parser errors (e.g. nonlocal,
dicts, strings or the @-operator for matrix multiplication).

Listed here are productions with NUMBER, NAME, ”symbols”, NEWLINE, INDENT,
DEDENT or STRING as terminals, nonterminals are defined on the left side of the -> arrow.

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

**Notes**

An input has to be organised like the template, so best is to fill it out with
the values for hh, hm, ph, pm like in a ZPAQ configuration to define the size of
H and M in hcomp and pcomp sections. In the dict which serves for calculation of
n (i.e. number of context mixing components) you have to specify the components
as in a ZPAQ configuration file, arguments are documented in the specification
(see `--info-zpaq` for link).

Only valid Python programmes without exceptions are supported as input, so run
them standalone before compiling.
For the arrays on top of H or M there is no boundary check, please make sure
the Python version works correct. If you need a ringbuffer on H or M, you have
to use `% len(hH)` or `&((1<<hh)-1)` and can not rely on integer overflows or the
modulo-array-length operation on indices in H or M like in plain ZPAQL because
H is expanded to contain the stack (and also due to the lack of overflows when
running the plain Python script)

Only positive 32-bit integers can be used, no strings, lists, arbitrary big
numbers, classes, closures and (function) objects.

**Input File**

Must be a runnable Python 3.5 file in form of the template and encoded as UTF-8
without a BOM (Byte-Order-Mark). The definitions at the beginning should be
altered and own code inserted only behind. The other two editable sections can
refer to definitions in the first section.

        Template Sections (--emit-template > source.py)         |   Editable?
----------------------------------------------------------------|--------------
  Definition of the ZPAQ configuration header data (memory size, context mixing components) and optionally functions and variables used by both hcomp and pcomp                        |      yes
  API functions for input and output, initialization of memory  |       no
  function hcomp and associated global variables and functions  |      yes
  function pcomp and associated global variables and functions  |      yes
  code for standalone execution of the Python file analog to running a ZPAQL configuration with zpaqd `r [cfg] p|h`          |       no

**Exposed API**

The 32- or 8-bit memory areas H and M are available as arrays `hH`, `pH`, `hM`, `pM`
depending on being a hcomp or pcomp section with size `2**hh` , `2**hm` , `2**ph`,
`2**pm` defined in the header as available constants hh, hm, ph, pm.
There is support for `len(hH)`, `len(pH)`, `len(hM)`, `len(pM)` instead of calculating
`2**hh`. But in general len() is not supported, see `len_hH()` below for dynamic
arrays. `NONE` is a shortcut for 0 - 1 = 4294967295.

      Other functions       |                   Description
----------------------------|--------------------------------------------------
c = read_b()                | Read one input byte, might leave VM execution and return to get next
push_b(c)                   | Put read byte c back, overwrites if already present (no buffer)
c = peek_b()                | Read but do not consume next byte, might leave VM execution and return to get next
out(c)                      | In pcomp: write c to output stream
error()                     | Execution fails with ”Bad ZPAQL opcode”
aref = alloc_pH(asize), …   | Allocate an array of size asize on pH/pM/hH/hM
aref = array_pH(intaddr), … | Cast an integer address back to a reference
len_pH(aref), …             | Get the length of an array in pH/pM/hH/hM
free_pH(aref), …            | Free the memory in pH/pM/hH/hM again by
                            | destructing the array

If backend implementations `addr_alloc_pH(size)`, `addr_free_pH(addr)`, … are
defined then dynamic memory management is available though the API functions
`alloc_pM` and `free_pM`. The cast `array_pH(numbervar)` is sometimes needed when the
array reference is passed between functions because then it is just treated as
integer again because no boxed types are used in general.

The template provides sample implementations of `addr_alloc_pM`, `addr_free_pM` , ….
The returned pointer is expected to point at the first element of the array. One
entry before the first element is used to store whether this memory section is
free or not. Before that the length of the array is store, i.e.
H[arraypointer-2] for arrays in H and the four bytes
M[arraypointer-5]…M[arraypointer-2] of the 32-bit length for arrays in M.

The last addressable starting point for any list is 2147483647 == (1<<31) - 1
because the compiler uses the 32nd bit to distinguish between pointers to M/H.

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
      0: "cm 19 22",  # context table size 2*19 with partly decoded byte as 9 bit hash xored with the context, count limit 22
      1: "cm 19 22",
      2: "mix2 1 0 1 30 0",  # will mix 0 and 1 together, context table size 2**1 with and-0 masking of the partly decoded byte which is added to the context, learning rate 30
    })

Each component i gets its context input from the entry in H[i] after each run of
the hcomp function, which is called for each input byte of the preprocessed data,
which either is to be stored through arithmetic coding in compression phase
or is retrieved through decoding in decompression phase with following
postprocessing done by calls of the pcomp function.

Then we specify a preprocessor:

    pcomp_invocation = "./simple_rle"

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
    last_value = 0
    last_counter = 0
    
    def hcomp(c):  # pcomp bytecode is passed first (or 0 if there is none)
      global at_counter, last_value, last_counter
      if at_counter:
        last_counter = c
      else:
        last_value = c
      # first part of the context for the first CM is the byte replicated and
      # the second part is whether we are at a counter (then we predict for a byte) or vice versa
      hH[0] = (last_value << 1) + at_counter  # at_counter will occupy one bit, therefore shift
      hH[0] <<= 9  # again shift to the side because of the xor with the partially decoded byte
      # second CM same but uses the counter for prediction
      hH[1] = (last_counter << 1) + at_counter
      hH[1] <<= 9
      hH[2] = at_counter + 0  # context for mixer: is at counter (1) or not (0)
      at_counter = not at_counter

We need to compile again before we run the final ZPAQ configuration file:

    ./zpaqlpy rle_model.py
    ./zpaqd c rle_model.cfg archive.zpaq FILE FILE FILE

zpaqd needs to have simple_rle in the same folder because we specified `pcomp_invocation = "./simple_rle"`

