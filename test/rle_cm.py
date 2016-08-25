#!/usr/bin/env python3

# simple features: if-else, while, for
# extra features: functions, global vs local function variables, own arrays and hcomp with return instead of hH or pM etc, dict?

### BEGIN OF EDITABLE SECTION - do not remove this marker or place something before and after it

hh = 0
hm = 0
ph = 0
pm = 24
n = len({
0: "cm 18 255"  # SIZE 2^18, LIMIT 255
})
pcomp_invocation = "./rle c"  # information for zpaqd about preprocessor invocation

# put commonly used functions and variables here
# as pcomp and hcomp do only include their own definitions


### END OF EDITABLE SECTION - do not remove this marker or place something before and after it
import sys, array, argparse
input_buf = []
output = []
NONE = 4294967295
input_c = NONE

def out(a):
  output.append(a)

def read_b():
  global input_c, input_buf
  if input_c == NONE:  # was already consumed
    if len(input_buf) == 0:
      return NONE
    a = input_buf.pop(0)
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


hH = array.array('L', [0 for x in range(0, 2**hh)])
hM = array.array('B', [0 for x in range(0, 2**hm)])

pH = array.array('L', [0 for x in range(0, 2**ph)])
pM = array.array('B', [0 for x in range(0, 2**pm)])

### BEGIN OF EDITABLE SECTION - do not remove this marker, may only use own variables and functions

mode = 0

def hcomp(c):
  global mode
  c <<= 1
  if not mode:
    mode = 1
  else:
    c += 1
    mode = 0
  c<<= 9
  hH[0] = c

pass
### END OF EDITABLE SECTION - do not remove this marker and the pass statement before

### BEGIN OF EDITABLE SECTION - do not remove this marker, may only use own variables and functions + read_b(), peek_b(), push_b(), out(b)

case = 0
last = 0

def pcomp(c):  # passing c is like having c = read_b() as first line
  global case, last
  if case == 0:  # c is byte to load
    case = 1
    last = c
  else:  # write out content of last c times
    case = 0
    while c > 0:
      c-= 1
      out(last)

pass
### END OF EDITABLE SECTION - do not remove this marker and the pass statement before


import argparse
parser = argparse.ArgumentParser()
parser.add_argument('method', help='run either hcomp or pcomp on each byte of the input\nfor hcomp output will be pairs of input and contexts', choices=['hcomp', 'pcomp'])
parser.add_argument('input', nargs='?', type=argparse.FileType('rb'), default=sys.stdin, help='input file')
parser.add_argument('output', nargs='?', type=argparse.FileType('wb'), default=sys.stdout, help='output file')
args = parser.parse_args()
input_buf = list(args.input.read())  # @TODO: more than one input file, separated with NONE (only pcomp?)
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
    input_last_a = input_buf.pop(0)
    hcomp(input_last_a)  # @TODO: hcomp also gets -1 after EOS like pcomp?
    print_hcomp_status()
elif args.method == 'pcomp':
  while len(input_buf) > 0:
    input_c = NONE
    pcomp(input_buf.pop(0))
  input_c = NONE
  pcomp(NONE)
  try:
    args.output.write(bytes(output))
  except:  # stdout
    args.output.buffer.write(bytes(output))
