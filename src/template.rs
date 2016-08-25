
pub static EMPTY_SOURCE: &'static str = r####"#!/usr/bin/env python3
# Copyright (C) 2016 Kai Lüke kailueke@riseup.net
# This program comes with ABSOLUTELY NO WARRANTY and is free software, you are welcome to redistribute it
# under certain conditions, see https://www.gnu.org/licenses/gpl-3.0.en.html
### BEGIN OF EDITABLE SECTION - do not remove the markers or place anything before/after them
# Author: NAME EMAIL

# definition of the array sizes and the context mixing linear tree
hh = 0  # size of hH[] is 2**hh
hm = 0
ph = 0
pm = 0
n = len({  # can also be an empty {}, then hcomp won't be included and (preprocessed) data is just stored and not arithmetically coded
0: "cm 19 22",  # means context model SIZE=2**19, COUNTLIMIT=22 (see --info-zpaq for link to ZPAQ spec)
})
pcomp_invocation = ""  # information for zpaqd about preprocessor invocation,
#                        like you would execute it in the shell, passed additional parameters
#                        at the end will be <inputfile> <outputfile>

# put shared functions and variables of pcomp and hcomp here,
# then they are copied into the hcomp and pcomp section before compilation


# dynamic memory allocation on H or M

def get32_hM(addr):
  return (((((hM[addr] << 8) + hM[addr+1]) << 8) + hM[addr+2]) << 8) + hM[addr+3]

def set32_hM(addr, value):
  hM[addr+3] = value % 256
  value >>= 8
  hM[addr+2] = value % 256
  value >>= 8
  hM[addr+1] = value % 256
  value >>= 8
  hM[addr] = value % 256

def get32_pM(addr):
  return (((((pM[addr] << 8) + pM[addr+1]) << 8) + pM[addr+2]) << 8) + pM[addr+3]

def set32_pM(addr, value):
  pM[addr+3] = value % 256
  value >>= 8
  pM[addr+2] = value % 256
  value >>= 8
  pM[addr+1] = value % 256
  value >>= 8
  pM[addr] = value % 256


def addr_alloc_pM(size):
  start = 0  # each block is either of type free (0) or allocated (1), starting with (size0…3, type) <- 5
  if get32_pM(start) == 0 and pM[start+4] == 0 and get32_pM(start+5) == 0 and pM[start+9] == 0:  # first run
    if len(pM)-start-10 < size:  # also 5 bytes for ending entry (0,0)
      error()
    set32_pM(start, size)
    pM[start+4] = 1
    set32_pM(start+size+5, len(pM)-start-size-10)
    pM[start+size+6] = 0
    return start+5  # return pointer to first element
  pos = start
  while len(pM)-pos-10 >= size: # also needs 5 bytes for ending entry
    # block is free? and (block is exact size or (block is bigger and following is free) or block is 5 bigger to append free block)
    pos_size = get32_pM(pos)
    if pM[pos+4] == 0 and (pos_size == size or (
        pos_size > size and pos+9+pos_size < len(pM) and pM[pos+9+pos_size] == 0) or pos_size >= size+5):  # found
      if pos_size > size: # handle rest
        if pos+9+pos_size < len(pM) and pM[pos+9+pos_size] == 0: # merge rest with next block
          # span this new free block over the next one (fetching it's size)
          set32_pM(pos+5+size, pos_size-size + get32_pM(pos+5+pos_size))  # -5 (this new free block's header) + 5 (next header)
          pM[pos+9+size] = 0
        else: # new free block inside current block after allocated block
          set32_pM(pos+5+size, pos_size-size-5)  # 5 is this free block's header
          pM[pos+9+size] = 0
      set32_pM(pos, size)
      pM[pos+4] = 1  # allocated
      return pos+5  # return pointer to first element
    # no adequate block, skip
    pos += 5 + pos_size
  error()



def addr_alloc_pH(size):
  start = 0  # each block is either of type free (0) or allocated (1), starting with (size, type)
  if pH[start] == 0 and pH[start+1] == 0 and pH[start+2] == 0 and pH[start+3] == 0:  # first run
    if len(pH)-start-4 < size:  # also 2 bytes for ending entry (0,0)
      error()
    pH[start] = size
    pH[start+1] = 1
    pH[start+size+2] = len(pH)-start-size-4
    pH[start+size+3] = 0
    return start+2  # return pointer to first element
  pos = start
  while len(pH)-pos-4 >= size: # also needs 2 bytes for ending entry
    # block is free? and (block is exact size or (block is bigger and following is free) or block is 2 bigger to append free block)
    if pH[pos+1] == 0 and (pH[pos] == size or (pH[pos] > size and pos+3+pH[pos] < len(pH) and pH[pos+3+pH[pos]] == 0) or pH[pos] >= size+2):  # found
      if pH[pos] > size: # handle rest
        if pos+3+pH[pos] < len(pH) and pH[pos+3+pH[pos]] == 0: # merge rest with next block
          # span this new free block over the next one (fetching it's size)
          pH[pos+2+size] = pH[pos]-size + pH[pos+2+pH[pos]]  # -2 (this new free block's header) + 2 (next header)
          pH[pos+3+size] = 0
        else: # new free block inside current block after allocated block
          pH[pos+2+size] = pH[pos]-size-2 # 2 is this free block's header
          pH[pos+3+size] = 0
      pH[pos] = size
      pH[pos+1] = 1  # allocated
      return pos+2  # return pointer to first element
    # no adequate block, skip
    pos += 2 + pH[pos]
  error()


def addr_alloc_hH(size):
  start = n  # beginn after H[0]…H[n-1], each block is either of type free (0) or allocated (1), starting with (size, type)
  if hH[start] == 0 and hH[start+1] == 0 and hH[start+2] == 0 and hH[start+3] == 0:  # first run
    if len(hH)-start-4 < size:  # also 2 bytes for ending entry (0,0)
      error()
    hH[start] = size
    hH[start+1] = 1
    hH[start+size+2] = len(hH)-start-size-4
    hH[start+size+3] = 0
    return start+2  # return pointer to first element
  pos = start
  while len(hH)-pos-4 >= size: # also needs 2 bytes for ending entry
    # block is free? and (block is exact size or (block is bigger and following is free) or block is 2 bigger to append free block)
    if hH[pos+1] == 0 and (hH[pos] == size or (hH[pos] > size and pos+3+hH[pos] < len(hH) and hH[pos+3+hH[pos]] == 0) or hH[pos] >= size+2):  # found
      if hH[pos] > size: # handle rest
        if pos+3+hH[pos] < len(hH) and hH[pos+3+hH[pos]] == 0: # merge rest with next block
          # span this new free block over the next one (fetching it's size)
          hH[pos+2+size] = hH[pos]-size + hH[pos+2+hH[pos]]  # -2 (this new free block's header) + 2 (next header)
          hH[pos+3+size] = 0
        else: # new free block inside current block after allocated block
          hH[pos+2+size] = hH[pos]-size-2 # 2 is this free block's header
          hH[pos+3+size] = 0
      hH[pos] = size
      hH[pos+1] = 1  # allocated
      return pos+2  # return pointer to first element
    # no adequate block, skip
    pos += 2 + hH[pos]
  error()

def addr_alloc_hM(size):
  start = 0  # beginn after H[0]…H[n-1], each block is either of type free (0) or allocated (1), starting with (size0…3, type) <- 5
  if get32_hM(start) == 0 and hM[start+4] == 0 and get32_hM(start+5) == 0 and hM[start+9] == 0:  # first run
    if len(hM)-start-10 < size:  # also 5 bytes for ending entry (0,0)
      error()
    set32_hM(start, size)
    hM[start+4] = 1
    set32_hM(start+size+5, len(hM)-start-size-10)
    hM[start+size+6] = 0
    return start+5  # return pointer to first element
  pos = start
  while len(hM)-pos-10 >= size: # also needs 5 bytes for ending entry
    # block is free? and (block is exact size or (block is bigger and following is free) or block is 5 bigger to append free block)
    pos_size = get32_hM(pos)
    if hM[pos+4] == 0 and (pos_size == size or (
        pos_size > size and pos+9+pos_size < len(hM) and hM[pos+9+pos_size] == 0) or pos_size >= size+5):  # found
      if pos_size > size: # handle rest
        if pos+9+pos_size < len(hM) and hM[pos+9+pos_size] == 0: # merge rest with next block
          # span this new free block over the next one (fetching it's size)
          set32_hM(pos+5+size, pos_size-size + get32_hM(pos+5+pos_size))  # -5 (this new free block's header) + 5 (next header)
          hM[pos+9+size] = 0
        else: # new free block inside current block after allocated block
          set32_hM(pos+5+size, pos_size-size-5)  # 5 is this free block's header
          hM[pos+9+size] = 0
      set32_hM(pos, size)
      hM[pos+4] = 1  # allocated
      return pos+5  # return pointer to first element
    # no adequate block, skip
    pos += 5 + pos_size
  error()


def addr_free_pM(addr):
  last_free = NONE
  pos = 0  # 0 is start
  addr -= 5  # addr showed to first element, not block start
  pM[addr+4] = 0  # free
  while pos < addr:
    if pM[pos+4] == 0:  # of type 'free'?
      last_free = pos
    else:
      last_free = NONE
    pos += 5 + get32_pM(pos)
  # is next block of type 'free'?
  addr_size = get32_pM(addr)
  if addr+9+addr_size < len(pM) and pM[addr+9+addr_size] == 0: # merge with next block
    # span this block over the next one (fetching it's size)
    set32_pM(addr, addr_size + 5 + get32_pM(addr+5+addr_size)) # + 5 is header
  # is last block free?
  if last_free != NONE:
    # merge last with this one
    set32_pM(last_free, get32_pM(last_free) + 5 + addr_size)  # + 5 is header


def addr_free_pH(addr):  # expects a valid reference to a used block
  last_free = NONE
  pos = 0  # 0 is start
  addr -= 2  # addr showed to first element, not block start
  pH[addr+1] = 0  # free
  while pos < addr:
    if pH[pos+1] == 0:  # of type 'free'?
      last_free = pos
    else:
      last_free = NONE
    pos += 2 + pH[pos]
  # is next block of type 'free'?
  if addr+3+pH[addr] < len(pH) and pH[addr+3+pH[addr]] == 0: # merge with next block
    # span this block over the next one (fetching it's size)
    pH[addr] = pH[addr] + 2 + pH[addr+2+pH[addr]] # + 2 is header
  # is last block free?
  if last_free != NONE:
    # merge last with this one
    pH[last_free] = pH[last_free] + 2 + pH[addr] # + 2 is header


def addr_free_hH(addr):  # expects a valid reference to a used block
  last_free = NONE
  pos = n  # n is start
  addr -= 2  # addr showed to first element, not block start
  hH[addr+1] = 0  # free
  while pos < addr:
    if hH[pos+1] == 0:  # of type 'free'?
      last_free = pos
    else:
      last_free = NONE
    pos += 2 + hH[pos]
  # is next block of type 'free'?
  if addr+3+hH[addr] < len(hH) and hH[addr+3+hH[addr]] == 0: # merge with next block
    # span this block over the next one (fetching it's size)
    hH[addr] = hH[addr] + 2 + hH[addr+2+hH[addr]] # + 2 is header
  # is last block free?
  if last_free != NONE:
    # merge last with this one
    hH[last_free] = hH[last_free] + 2 + hH[addr] # + 2 is header

def addr_free_hM(addr):
  last_free = NONE
  pos = 0  # 0 is start
  addr -= 5  # addr showed to first element, not block start
  hM[addr+4] = 0  # free
  while pos < addr:
    if hM[pos+4] == 0:  # of type 'free'?
      last_free = pos
    else:
      last_free = NONE
    pos += 5 + get32_hM(pos)
  # is next block of type 'free'?
  addr_size = get32_hM(addr)
  if addr+9+addr_size < len(hM) and hM[addr+9+addr_size] == 0: # merge with next block
    # span this block over the next one (fetching it's size)
    set32_hM(addr, addr_size + 5 + get32_hM(addr+5+addr_size)) # + 5 is header
  # is last block free?
  if last_free != NONE:
    # merge last with this one
    set32_hM(last_free, get32_hM(last_free) + 5 + addr_size)  # + 5 is header



pass
### END OF EDITABLE SECTION - do not remove the markers or place anything before/after them
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

def hcomp(c):  # pcomp bytecode is passed first (or 0 if there is none)
  pass  # having only pass in hcomp means that this whole section won't be included
  # add code here for computation of hH[0] … hH[n-1]



pass
### END OF EDITABLE SECTION - do not remove this marker and the pass statement before

### BEGIN OF EDITABLE SECTION - do not remove this marker, may only use own variables and functions beside API and those of the first section


# place global variables of pcomp and custom functions into this section

def pcomp(c):  # passing c is like having c = read_b() as first line
  pass  # having only pass in pcomp means that this whole section won't be included
  # add code here which writes output via out(x)



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
"####;

