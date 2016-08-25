#!/usr/bin/env python3
# Copyright (C) 2016 Kai Lüke kailueke@riseup.net
# This program comes with ABSOLUTELY NO WARRANTY and is free software, you are welcome to redistribute it
# under certain conditions, see https://www.gnu.org/licenses/gpl-3.0.en.html
### BEGIN OF EDITABLE SECTION - do not remove this marker or place something before and after it

# definition of the array sizes and the context mixing linear tree
hh = 0  # size of hH[] is 2**hh
hm = 0
ph = 0
pm = 24
n = len({  # can also be an empty {}, then hcomp won't be included and (preprocessed) data is just stored and not arithmetically coded
0: "icm 12",  # means indirect context model SIZE=2**12 (see --info-zpaq for link to ZPAQ spec)
})
pcomp_invocation = "./lzpre c"  # information for zpaqd about preprocessor invocation,
#                        like you would execute it in the shell, passed additional parameters
#                        at the end will be <inputfile> <outputfile>

# put shared functions and variables of pcomp and hcomp here,
# then they are copied into the hcomp and pcomp section before compilation

pass
### END OF EDITABLE SECTION - do not remove this marker or place something before and after it
# ***Exposed API***
# c = read_b()
# push_b(c)
# c = peek_b()
# out(c)
# error()
# hH, pH, hM, pM as 32- and 8-bit arrays with the defined size 2**hh, … and support for len(hH), …
# hh, hm, ph, pm and n are also available as constants
# arrayvar = alloc_pH(arraysize)  # if backend implementations addr_alloc_pH(size), addr_free_pH(addr) … are defined
# arrayvar = array_pH(numbervar)  # cast needed when passed between functions
# len_pH(arrayvar)
# free_pH(arrayvar)
# … analog for pM, hH, hM

import sys, array, argparse
from collections import deque
input_buf = []
output = deque([])
NONE = 4294967295
input_c = NONE-1

def out(a):
  if cmpbuf is not None:
    expected = cmpbuf.popleft()
    if a != expected:
      import ipdb; ipdb.set_trace()
  output.append(a)

def read_b():
  global input_c, input_buf, input_last_a
  if input_c == NONE-1:  # was already consumed
    if len(input_buf) == 0:
      raise WouldNotBeReached
    a = input_buf.popleft()
    print_hcomp_status()
    input_last_a = a
    return a
  else:
    tmp = input_c
    input_c = NONE-1
    return tmp


def peek_b():
  global input_c
  if input_c == NONE-1:
    push_b(read_b())
  return input_c

def push_b(c):
  """can only be executed once and will overwrite otherwise"""
  global input_c
  input_c = c

def error():
  raise Exception("error() invoked (zpaq execution will fail with: Bad ZPAQL opcode)")

hH = array.array('L', [0 for x in range(0, 2**hh)])
hM = array.array('B', [0 for x in range(0, 2**hm)])

pH = array.array('L', [0 for x in range(0, 2**ph)])
pM = array.array('B', [0 for x in range(0, 2**pm)])


def alloc_pM(size):
  return VirtArray(pM, addr_alloc_pM(size), size)
def alloc_pH(size):
  return VirtArray(pH, addr_alloc_pH(size), size)
def alloc_hH(size):
  return VirtArray(hH, addr_alloc_hH(size), size)
def alloc_hM(size):
  return VirtArray(hM, addr_alloc_hM(size), size)
def free_pM(va):
  if va.addr == NONE:
    raise Exception("double free (not visible in zpaq execution)")
  addr_free_pM(va.addr)
  va.addr = NONE
def free_pH(va):
  if va.addr == NONE:
    raise Exception("double free (not visible in zpaq execution)")
  addr_free_pH(va.addr)
  va.addr = NONE
def free_hH(va):
  if va.addr == NONE:
    raise Exception("double free (not visible in zpaq execution)")
  addr_free_hH(va.addr)
  va.addr = NONE
def free_hM(va):
  if va.addr == NONE:
    raise Exception("double free (not visible in zpaq execution)")
  addr_free_hM(va.addr)
  va.addr = NONE

# casting addresses which where written itself into an pH/hH entry back to array objects
array_pH = lambda addr: addr if type(addr) is VirtArray else VirtArray(pH, addr, pH[addr-2])
array_pM = lambda addr: addr if type(addr) is VirtArray else VirtArray(pM, addr, get32_pM(addr-5))
array_hH = lambda addr: addr if type(addr) is VirtArray else VirtArray(hH, addr, hH[addr-2])
array_hM = lambda addr: addr if type(addr) is VirtArray else VirtArray(hM, addr, get32_hM(addr-5))
len_hM = len_pM = len_pH = len_hH = lambda va: va.size

class VirtArray:
  addr = None  # addr in array for index 0
  array = None  # one of hH, hM, pH, pM
  size = None
  def __init__(self, array, addr, size):
    self.array = array
    self.addr = addr
    self.size = size
    assert self.size < 2147483648, "address too big, 32. bit is used to distinguish between H and M"
  def __getitem__(self, key):
    return self.array[self.addr+key]
  def __setitem__(self, key, item):
    self.array[self.addr+key] = item.addr if type(item) is VirtArray else item
  def __len__(self):
    raise Exception("instead of len() use one of len_hM, len_pM, len_pH or len_hH")
  def __str__(self):
    return str(self.array[self.addr:self.addr+self.size])


pass
### BEGIN OF EDITABLE SECTION - do not remove this marker, may only use own variables and functions  beside API and those of the first section

# place global variables of hcomp and custom functions into this section

# Ported to Python by Kai Lüke, 2016, from lz1.cfg:
#  (C) 2011 Dell Inc. Written by Matt Mahoney
#  Licensed under GPL v3, http://www.gnu.org/copyleft/gpl.html)

# (state: 0=init, 1=expect LZ77 literal or match code,
# 2..4=expect n-1 offset bytes,
# 5..68=expect n-4 literals)

h_state = 5
first_run = True

def hcomp(c):  # pcomp bytecode is passed first (or 0 if there is none)
  # having only pass in hcomp means that this whole section won't be included
  # add code here for computation of hH[0] … hH[n-1]
  global h_state, first_run
  if first_run:
    first_run = False  # skip pcomp bytecode
    if c == 0:
      return
    if c == 1:
      c = read_b()
      c += read_b()*256 # read length
      while c > 0:
        hH[0] = read_b()
        c -= 1
      return
  if h_state == 1:  # (expect code ccxxxxxx as input) (cc is number of offset bytes following) (00xxxxxx means x+1 literal bytes follow)
    a=c
    a>>= 6
    a&= 3
    if a > 0:
      a += 1
      h_state = a
      a = c
      a>>= 3
      hH[0] = ( a + 512) * 773
    else:
      a = c
      a&= 63
      a+= 5
      h_state = a
      a = c
      hH[0] = ( a + 512) * 773
  elif h_state == 5: # (end of literal) # + #  (init)
    h_state = 1
    hH[0] = 0
  else: # (literal or offset)
    if h_state > 5:
      hH[0] = (c + 512) * 773
    h_state -= 1
  c = h_state
  if h_state > 5:
    c = 5
  hH[0] = (hH[0] + c + 512) * 773

pass
### END OF EDITABLE SECTION - do not remove this marker and the pass statement before

### BEGIN OF EDITABLE SECTION - do not remove this marker, may only use own variables and functions beside API and those of the first section

# place global variables of pcomp and custom functions into this section

# Ported to Python by Kai Lüke, 2016, from C++ decode section of lzpre.cpp:
#  (C) 2011 Dell Inc. Written by Matt Mahoney
#  Licensed under GPL v3, http://www.gnu.org/copyleft/gpl.html

# Input format:
#    00llllll: literal of length lllllll=1..64 to follow
#    01lllooo oooooooo: length lll=5..12, offset o=1..2048
#    10llllll oooooooo oooooooo: l=1..64 offset=1..65536
#    11llllll oooooooo oooooooo oooooooo: 1..64, 1..2^24)

i = 0  # position in 16M output buffer
state = 0
leng = 0  # length of match or literal
off = 0  # offset of match back from i
BUFSIZE_max = (1<<24) - 1

def pcomp(c):  # passing c is like having c = read_b() as first line
  # having only pass in pcomp means that this whole section won't be included
  # add code here which writes output via out(x)
  global i, state, leng, off
  if c == NONE:  # restart
    i = 0
    state = 0
    leng = 0
    off = 0
    return
  if state == 0: # expecting a literal or match code
    state = 1+(c>>6)
    if state == 1: # literal
      off = 0
      leng = c+1
    elif state==2: # short match
      off = c&7
      leng = (c>>3)-3
    else:
      off = 0
      leng = (c&63)+1  # match
  elif state == 1: # decoding a literal with leng bytes remaining
    out(c)
    pM[i&BUFSIZE_max] = c
    i += 1
    leng -= 1
    if leng == 0:
      state = 0
  elif state > 2: # state==3, state==4: expecting 2,3 match offset bytes
    off = off<<8|c
    state -= 1
  else:  # state == 2, expecting last offset byte of a match code
    off = off<<8|c
    off = i-off-1
    while leng:
      c=pM[off&BUFSIZE_max]
      pM[i&BUFSIZE_max]=c
      i += 1
      off += 1
      out(c)
      leng -= 1
    state = 0

pass
### END OF EDITABLE SECTION - do not remove this marker and the pass statement before

class WouldNotBeReached(Exception):
  """used for handling EOF in read_b() as execution does not continue after last byte (or end-of-segment in pcomp) is consumed"""
  pass

def finish_output():
  global output
  try:
    args.output[0].write(bytes(output))
  except:  # stdout
    args.output[0].buffer.write(bytes(output))
  output = deque([])
  if len(args.output) > 1:
    args.output.pop(0)

import argparse
parser = argparse.ArgumentParser()
parser.add_argument('method', help='run either hcomp or pcomp on each byte of the input\nfor hcomp output will be pairs of input and contexts', choices=['hcomp', 'pcomp'])
parser.add_argument('input', nargs='?', type=argparse.FileType('rb'), default=sys.stdin, help='input file')
parser.add_argument('--append', type=argparse.FileType('rb'), dest='addseg', default=[], metavar='FILE', action='append', help='additional input files')
parser.add_argument('--compare', type=argparse.FileType('rb'), dest='compare', default=None, metavar='EXPECTEDFILE', help='compare pcomp output and run ipdb for mismatch')
parser.add_argument('output', nargs='*', type=argparse.FileType('wb'), default=[sys.stdout], help='output file')
args = parser.parse_args()
cmpbuf = None
if args.compare:
  cmpbuf = deque(args.compare.read())
input_buf = deque(args.input.read())
if args.method == 'pcomp':
  input_buf.append(NONE)  # end of segment
for additional_segment in args.addseg:
  input_buf.extend(additional_segment.read())
  if args.method == 'pcomp':
    input_buf.append(NONE)
input_last_a = None

def print_hcomp_status():
  global input_last_a
  if input_last_a is None:
    return
  line = '{}: {}\n'.format(input_last_a, list(hH[:n]))
  if args.method == 'pcomp' and input_last_a == NONE:
    finish_output()
  input_last_a = None
  if args.method == 'hcomp':
    try:  # stdout
      args.output[0].write(line)
    except:
      args.output[0].write(bytes(line, 'utf-8'))

if args.method == 'hcomp':
  while len(input_buf) > 0:
    input_c = NONE-1
    input_last_a = input_buf.popleft()
    try:
      hcomp(input_last_a)
    except WouldNotBeReached:
      pass
    print_hcomp_status()
elif args.method == 'pcomp':
  while len(input_buf) > 0:
    input_c = NONE-1
    input_last_a = input_buf.popleft()
    try:
      pcomp(input_last_a)
    except WouldNotBeReached:
      pass
    print_hcomp_status()


