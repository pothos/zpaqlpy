#!/usr/bin/env python3
# Copyright (C) 2016 Kai Lüke kailueke@riseup.net
# This program comes with ABSOLUTELY NO WARRANTY and is free software, you are welcome to redistribute it
# under certain conditions, see https://www.gnu.org/licenses/gpl-3.0.en.html
### BEGIN OF EDITABLE SECTION - do not remove this marker or place something before and after it

# definition of the array sizes and the context mixing linear tree
hh = 2  # size of hH[] is 2**hh
hm = 0
ph = 0
pm = 0
n = len({  # can also be an empty {}, then hcomp won't be included and (preprocessed) data is just stored and not arithmetically coded
0: "cm 19 22",  # context table size 2*19 with partly decoded byte as 9 bit hash xored with the context, count limit 22
1: "cm 19 22",
2: "mix2 1 0 1 30 0",  # will mix 0 and 1 together, context table size 2**1 with and-0 masking of the partly decoded byte which is added to the context, learning rate 30
})
pcomp_invocation = "./simple_rle"  # information for zpaqd about preprocessor invocation,
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
  if va.array is not pM:
    raise Exception("wrong type")
  addr_free_pM(va.addr)
  va.addr = NONE
def free_pH(va):
  if va.addr == NONE:
    raise Exception("double free (not visible in zpaq execution)")
  if va.array is not pH:
    raise Exception("wrong type")
  addr_free_pH(va.addr)
  va.addr = NONE
def free_hH(va):
  if va.addr == NONE:
    raise Exception("double free (not visible in zpaq execution)")
  if va.array is not hH:
    raise Exception("wrong type")
  addr_free_hH(va.addr)
  va.addr = NONE
def free_hM(va):
  if va.addr == NONE:
    raise Exception("double free (not visible in zpaq execution)")
  if va.array is not hM:
    raise Exception("wrong type")
  addr_free_hM(va.addr)
  va.addr = NONE

# casting addresses which where written itself into an pH/hH entry back to array objects
array_pH = lambda addr: (addr if addr.array is pH else error()) if type(addr) is VirtArray else VirtArray(pH, addr, pH[addr-2])  # wrong type error?
array_pM = lambda addr: (addr if addr.array is pM else error()) if type(addr) is VirtArray else VirtArray(pM, addr, get32_pM(addr-5))  # wrong type error?
array_hH = lambda addr: (addr if addr.array is hH else error()) if type(addr) is VirtArray else VirtArray(hH, addr, hH[addr-2])  # wrong type error?
array_hM = lambda addr: (addr if addr.array is hM else error()) if type(addr) is VirtArray else VirtArray(hM, addr, get32_hM(addr-5))  # wrong type error?
len_hM = lambda va: va.size if va.array is hM else error() # wrong type
len_pM = lambda va: va.size if va.array is pM else error() # wrong type
len_pH = lambda va: va.size if va.array is pH else error() # wrong type
len_hH = lambda va: va.size if va.array is hH else error() # wrong type

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
### BEGIN OF EDITABLE SECTION - do not remove this marker, may only use own variables and functions beside API and those of the first section


# place global variables of hcomp and custom functions into this section

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



pass
### END OF EDITABLE SECTION - do not remove this marker and the pass statement before

### BEGIN OF EDITABLE SECTION - do not remove this marker, may only use own variables and functions beside API and those of the first section


# place global variables of pcomp and custom functions into this section

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

