#!/usr/bin/env python3
# Copyright (C) 2016 Kai Lüke kailueke@riseup.net
# This program comes with ABSOLUTELY NO WARRANTY and is free software, you are welcome to redistribute it
# under certain conditions, see https://www.gnu.org/licenses/gpl-3.0.en.html
### BEGIN OF EDITABLE SECTION - do not remove this marker or place something before and after it

# definition of the array sizes and the context mixing linear tree
hh = 4  # size 2**hh of hH[] is enough for the 13 context entries
hm = 17  # increase for image width > (2**17)/3
ph = 0
pm = 0
n = len({
0: "cm 19 22 (L, color)",  # SIZE 2**19, COUNTLIMIT 22
1: "cm 19 22 (U, color)",
2: "cm 19 22 (UL, color)",
3: "cm 19 22 (UR, color)",
4: "cm 19 22 (avg L UL U UR, color)",
5: "icm 10 (L, color)",
6: "icm 10 (U, color)",
7: "cm 19 22 (c)",
8: "const 160",
9: "mix 11 0 9 60 255", # sizebits j m rate mask. adaptive mixing of predictions 0 to 9
10: "icm 10 (avg l u, color)",
11: "isse 24 10 (hash of UL U L c color)",
12: "mix2 2 9 11 90 0 (color)",  # adaptive mixing of first mixer and isse
})
pcomp_invocation = "./subtract_green"  # information for zpaqd about preprocessor invocation,
#                        like you would execute it in the shell, passed additional parameters
#                        at the end will be <inputfile> <outputfile>


# put commonly used functions and variables here
# as pcomp and hcomp do only include their own definitions


### END OF EDITABLE SECTION - do not remove this marker or place something before and after it
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
  """zpaq execution will fail with: Bad ZPAQL opcode"""
  raise Exception

hH = array.array('L', [0 for x in range(0, 2**hh)])
hM = array.array('B', [0 for x in range(0, 2**hm)])

pH = array.array('L', [0 for x in range(0, 2**ph)])
pM = array.array('B', [0 for x in range(0, 2**pm)])

### BEGIN OF EDITABLE SECTION - do not remove this marker, may only use own variables and functions


def read_after_whitespace():
  while True:
    c = read_b()
    if c == 0x20 or c == 9 or c == 10 or c == 13: # skip whitespace
      continue
    elif c == 0x23: # skip comment line
     while c != 10 and c != 13:
       c = read_b()
    else:
      return c

line_len = 0  # (1+width)*3 for all three colors
total_color_bytes = 0

def read_after_header():
  global line_len, total_color_bytes
  c = read_after_whitespace() # skip over first delimiter
  z = c - 0x30
  line_len = 0
  while z < 10 and z >= 0:  # read width
    line_len *= 10
    line_len += z
    z = read_b() - 0x30
  total_color_bytes = line_len * 3
  line_len += 1  # increase by one to also have space for the upper left pixel
  line_len *= 3
  c = read_after_whitespace()
  z = c - 0x30
  height = 0
  while z < 10 and z >= 0:  # skip height
    height *= 10
    height += z
    z = read_b() - 0x30
  total_color_bytes *= height
  c = read_after_whitespace()
  z = c - 0x30
  maxc = 0
  while z < 10 and z >= 0:
    maxc *= 10
    maxc += z
    z = read_b() - 0x30
  if maxc > 255:
    error() # only three 8-bit RGB channels supported
  # skip single whitespace in z
  return read_b() # read first color byte

buf_pos = 0
color = 0
reading_header = True
color_bytes_read = 0
first_run = True

def hcomp(c):  # pcomp bytecode is passed first (or 0 if there is none)
  global buf_pos, reading_header, color, color_bytes_read, first_run
  if first_run:
    first_run = False  # skip pcomp bytecode
    if c == 0:
      return
    if c == 1:
      c = read_b()
      c += read_b()*256 # read length
      while c > 0:
        hH[0] = read_b() << 9  # try to save some space when encoding pcomp bytecode
        hH[5] = hH[0]
        c -= 1
      return
  color = (color + 1) % 3
  if reading_header and c == 0x50:
    c = read_b()
    if c == 0x36: # detected header P6
      c = read_after_header()
      color = 0  # red
      color_bytes_read = 0
      reading_header = False
  color_bytes_read += 1
  upper = hM[(buf_pos+4) % line_len]
  upper_left = hM[(buf_pos+1)%line_len]
  upper_right = hM[(buf_pos+7) % line_len]
  left = hM[(line_len+buf_pos-2) % line_len]  # fetch left pixel's relevant color part
  hM[buf_pos] = c
  buf_pos = (buf_pos + 1) % line_len # move history buffer pointer
  hH[0] = ((left << 2) + color) << 9
  hH[1] = ((upper << 2) + color) << 9
  hH[2] = ((upper_left << 2) + color) << 9
  hH[3] = ((upper_right << 2) + color) << 9
  hH[4] = (((upper_left + upper + upper_right + left)//4 << 2) + color) << 9
  hH[5] = ((left << 2) + color) << 9
  hH[6] = ((upper << 2) + color) << 9
  hH[7] = c << 9
  hH[10] = (((left+upper)//2 << 2) + color) << 9
  hH[11] = (((((((upper_left + 512) * 773) + upper + 512) * 773) + left + 512) * 773) + color + 512) * 773 + c # 4 order hash
  hH[12] = color
  if upper > upper_left:
    dH = upper - upper_left
  else:
    dH = upper_left - upper
  if left > upper_left:
    dV = left - upper_left
  else:
    dV = upper_left - left
  if (dH + dV) < 17: # threshold for simple smooth region detection
    hH[9] = (4 + color) << 8
  else:
    hH[9] = color << 8
  if color_bytes_read == total_color_bytes: # await next PNM image
    reading_header = True

pass
### END OF EDITABLE SECTION - do not remove this marker and the pass statement before

### BEGIN OF EDITABLE SECTION - do not remove this marker, may only use own variables and functions + read_b(), peek_b(), push_b(), out(b)

def pcomp_read_after_whitespace():
  while True:
    c = read_b()
    if c == 0x20 or c == 9 or c == 10 or c == 13: # skip whitespace
      out(c)
      continue
    elif c == 0x23: # skip comment line
     while c != 10 and c != 13:
       out(c)
       c = read_b()
     out(c)
    else:
      return c

def pcomp_read_after_header():
  c = pcomp_read_after_whitespace() # skip over first delimiter
  out(c)
  z = c - 0x30
  while z < 10 and z >= 0:  # read width
    c = read_b()
    out(c)
    z = c - 0x30
  c = pcomp_read_after_whitespace()
  out(c)
  z = c - 0x30
  while z < 10 and z >= 0:  # skip height
    c = read_b()
    out(c)
    z = c - 0x30
  c = pcomp_read_after_whitespace()
  out(c)
  z = c - 0x30
  while z < 10 and z >= 0:
    c = read_b()
    out(c)
    z = c - 0x30
  # skip single whitespace in z
  return read_b() # read first color byte

pcomp_reading_header = True
pcomp_color = 0
g = 0

def pcomp(c):  # passing c is like having c = read_b() as first line
  global pcomp_reading_header, pcomp_color, g
  if c == NONE:
    pcomp_reading_header = True
    return
  pcomp_color = (pcomp_color + 1) % 3
  if pcomp_reading_header and c == 0x50:
    out(c)
    c = read_b()
    if c == 0x36: # detected header P6
      out(c)
      c = pcomp_read_after_header()
      pcomp_color = 0  # red
      pcomp_reading_header = False
  if pcomp_color == 1:
    # wrong data
    out(c)
    pcomp_color = 0
  elif pcomp_color == 0:
    r = c
    c = read_b()
    out((r+c) % 256)
    out(c)
    g = c
    pcomp_color = 1
  elif pcomp_color == 2:
    out((c+g) % 256)

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
parser.add_argument('--append', type=argparse.FileType('rb'), dest='addseg', metavar='FILE', default=[], action='append', help='additional input files')
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
