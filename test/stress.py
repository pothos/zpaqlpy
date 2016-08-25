#!/usr/bin/env python3

# simple features: if-else, while, for
# extra features: functions, global vs local function variables, own arrays and hcomp with return instead of hH or pM etc, dict?

### BEGIN OF EDITABLE SECTION - do not remove this marker or place something before and after it

hh = 0
hm = 15
ph = 0
pm = 0
n = len({
0: "cm 16 22"  # SIZE 2^16, LIMIT 255
})
pcomp_invocation = ""  # information for zpaqd about preprocessor invocation

# put commonly used functions and variables here
# as pcomp and hcomp do only include their own definitions


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
  raise Exception

hH = array.array('L', [0 for x in range(0, 2**hh)])
hM = array.array('B', [0 for x in range(0, 2**hm)])

pH = array.array('L', [0 for x in range(0, 2**ph)])
pM = array.array('B', [0 for x in range(0, 2**pm)])

### BEGIN OF EDITABLE SECTION - do not remove this marker, may only use own variables and functions

line_len = 8
buf_pos = 110

reading_header = True

def read_5_times():
  read_b()
  read_b()
  read_b()
  y=peek_b()
  x=read_b()
  if x!=y:
    error()
  push_b(x)
  if x!= peek_b():
    error()
  x=read_b()
  if x!=y:
    error()
  return read_b()

def w(v):
  return v << 9

def calc(up, left):
  if True and left != left:
    error()
  if False or up != up:
    error()
  if left:
    l = left ** 2
    left = l // left
  else:
    up *= 87
    up //= 87
    if True:
      return w(up//4 + left//4)
    else:
      return (up//4 + left//4) << 9
  v = up//4 + left//4
  v <<= 9
  return v


def hcomp(c):
  if len(hH) != 2**hh:
    error()
  if 2**26 != 67108864:
    error()
  x = 0
  x = ~x
  x = not x
  if not x and x:
    error()
  global line_len, buf_pos, reading_header
  if reading_header:
    if c != 0x50 and 0 < read_b():
      read_b()
      z = read_b() - 0x30
      while z < 10 and z >= 0:
        line_len *= 10
        line_len += z
        z = read_b() - 0x30
      line_len *= 3
      z = read_b() - 0x30
      c = read_5_times()
      reading_header = False
  up = hM[buf_pos]
  left = hM[(line_len+buf_pos-3) % line_len]
  hM[buf_pos] = c
  buf_pos = (buf_pos + 1) % line_len
  hH[0] = calc(up, left)

pass
### END OF EDITABLE SECTION - do not remove this marker and the pass statement before

### BEGIN OF EDITABLE SECTION - do not remove this marker, may only use own variables and functions + read_b(), peek_b(), push_b(), out(b)

def pcomp(c):  # passing c is like having c = read_b() as first line
  pass

pass
### END OF EDITABLE SECTION - do not remove this marker and the pass statement before


import argparse
parser = argparse.ArgumentParser()
parser.add_argument('method', help='run either hcomp or pcomp on each byte of the input\nfor hcomp output will be pairs of input and contexts', choices=['hcomp', 'pcomp'])
parser.add_argument('input', nargs='?', type=argparse.FileType('rb'), default=sys.stdin, help='input file')
parser.add_argument('output', nargs='?', type=argparse.FileType('wb'), default=sys.stdout, help='output file')
args = parser.parse_args()
input_buf = deque(args.input.read())  # @TODO: more than one input file, separated with NONE (only pcomp?)
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
    hcomp(input_last_a)
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
