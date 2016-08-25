#!/usr/bin/env python3
### BEGIN OF EDITABLE SECTION - do not remove this marker or place something before and after it

# definition of the array sizes and the context mixing linear tree
hh = 10  # size of hH[] is 2**hh
hm = 10
ph = 10
pm = 10
n = len({  # can also be an empty {}, then hcomp won't be included and (preprocessed) data is just stored and not arithmetically coded
0: "cm 19 22",  # means context model SIZE=2**19, COUNTLIMIT=22 (see --info-zpaq for link to ZPAQ spec)
})
pcomp_invocation = ""  # information for zpaqd about preprocessor invocation,
#                        like you would execute it in the shell, passed additional parameters
#                        at the end will be <inputfile> <outputfile>

# put shared functions and variables of pcomp and hcomp here,
# then they are copied into the hcomp and pcomp section before compilation


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
### END OF EDITABLE SECTION - do not remove this marker or place something before and after it
import sys, array, argparse
from collections import deque
input_buf = []
output = deque([])
NONE = 4294967295
input_c = NONE

def out(a):
  output.append(a)

def read_b():
  global input_c, input_buf
  if input_c == NONE:  # was already consumed
    if len(input_buf) == 0:
      if args.method == 'hcomp':
        raise WouldNotBeReached
      return NONE
    a = input_buf.popleft()
    if args.method == 'hcomp':
      print_hcomp_status()
      global input_last_a
      input_last_a = a
    return a
  else:
    tmp = input_c
    input_c = NONE
    return tmp


def peek_b():
  global input_c
  if input_c == NONE:
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
    raise Exception("double free")
  addr_free_pM(va.addr)
  va.addr = NONE
def free_pH(va):
  if va.addr == NONE:
    raise Exception("double free")
  addr_free_pH(va.addr)
  va.addr = NONE
def free_hH(va):
  if va.addr == NONE:
    raise Exception("double free")
  addr_free_hH(va.addr)
  va.addr = NONE
def free_hM(va):
  if va.addr == NONE:
    raise Exception("double free")
  addr_free_hM(va.addr)
  va.addr = NONE

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
    self.array[self.addr+key] = item
  def __len__(self):
    raise Exception("instead of len() use one of len_hM, len_pM, len_pH or len_hH")
  def __str__(self):
    return str(self.array[self.addr:self.addr+self.size])

pass
### BEGIN OF EDITABLE SECTION - do not remove this marker, may only use own variables and functions


# place global variables of hcomp and custom functions into this section
#import random
i_x = 0
old_v = NONE
old_n = NONE
def hcomp(c):  # pcomp bytecode is passed first (or 0 if there is none)
  global i_x, old_v, old_n
  varray = NONE
  #try:
  i_x += 1
  i_x %= 256
  rr = (i_x + 7)*13 % 160
  #print("alloc:", rr)
  varray = alloc_hH(rr)
  #print("got loc", varray.addr, "filled with", i_x)
  if old_v != NONE:
    #print("freeing", old_v.addr)
    free_hH(old_v)
  old_v = old_n
  old_n = varray
  #  for i in range(0,len(varray)):
  i = 0
  while i < len_hH(varray):
      varray[i] = 1
      varray[i] += i_x -1
      if varray[i] != i_x:
        error() # assignment or access went wrong?
      i += 1
  #except:
  #  print(hH)
  #  print("varray", varray)
  #  raise Exception



pass
### END OF EDITABLE SECTION - do not remove this marker and the pass statement before

### BEGIN OF EDITABLE SECTION - do not remove this marker, may only use own variables and functions + read_b(), peek_b(), push_b(), out(b)


# place global variables of pcomp and custom functions into this section

#import random
i_x = 0
old_v = NONE
old_n = NONE
def pcomp(c):
  global i_x, old_v, old_n
  varray = NONE
  #try:
  i_x += 1
  i_x %= 256
  #rr = random.randint(0,160)
  rr = (i_x + 7)*13 % 160
  #  print("alloc:", rr)
  varray = alloc_pM(rr)
  #  print("got loc", varray.addr, "filled with", i_x)
  if old_v != NONE:
    #  print("freeing", old_v.addr)
    free_pM(old_v)
  old_v = old_n
  old_n = varray
  #  for i in range(0,len(varray)):
  i = 0
  while i < len_pM(varray):
      varray[i] = i_x
      if varray[i] != i_x:
        error()  # assignment or access went wrong?
      i += 1
  #except:
  #  print(pM)
  #  print("varray", varray)
  #  raise Exception



pass
### END OF EDITABLE SECTION - do not remove this marker and the pass statement before

class WouldNotBeReached(Exception): # @TODO: also needed for pcomp with more than one file?
  """used for handling EOF in read_b() of hcomp as execution does not continue because NONE after a segment is only used in pcomp"""
  pass

import argparse
parser = argparse.ArgumentParser()
parser.add_argument('method', help='run either hcomp or pcomp on each byte of the input\nfor hcomp output will be pairs of input and contexts', choices=['hcomp', 'pcomp'])
parser.add_argument('input', nargs='?', type=argparse.FileType('rb'), default=sys.stdin, help='input file')
parser.add_argument('output', nargs='?', type=argparse.FileType('wb'), default=sys.stdout, help='output file')
args = parser.parse_args()
input_buf = deque(args.input.read())  # @TODO: more than one input file, preprocessed (and separated with NONE for only pcomp)
input_last_a = None
def print_hcomp_status():
  global input_last_a
  if input_last_a is None:
    return
  line = '{}: {}\n'.format(input_last_a, list(hH[:n]))
  input_last_a = None
  try:  # stdout
    args.output.write(line)
  except:
    args.output.write(bytes(line, 'utf-8'))
if args.method == 'hcomp':
  while len(input_buf) > 0:
    input_c = NONE
    input_last_a = input_buf.popleft()
    try:
      hcomp(input_last_a)
    except WouldNotBeReached:
      pass
    print_hcomp_status()
elif args.method == 'pcomp':
  while len(input_buf) > 0:
    input_c = NONE
    pcomp(input_buf.popleft())
  input_c = NONE
  pcomp(NONE)
  try:
    args.output.write(bytes(output))
  except:  # stdout
    args.output.buffer.write(bytes(output))

