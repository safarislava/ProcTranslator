State Views
-
D0 - D7, A0 - A7, NZCV, PC
- A7 - stack pointer 

Instructions
-
- first byte - operator code
- next n bytes for operand descriptions, 
support concat for 64 bit

Operand Description 
-
- [7:5] - mode
- [4:2] - main register 
- [1:0] - offset register, support D4 - D7

Mode:
-
- 0x0 - #* - direct (next word after current instruction)
- 0x1 - D* - data register
- 0x2 - A* - address register 
- 0x3 - (A*) - indirect
- 0x4 - (A*)+ - indirect, post-increment
- 0x5 - -(A*) - indirect, pre-decrement
- 0x6 - (A*:D*) - indirect, with offset

Operator code:
- 
0x01 - MOV from, to
- from = {#* | D* | MEMORY}
- to = {D* | MEMORY}

0x02 - MOVA from, to
- from = {D* | A*}
- to = {D* | A*}
---
0x10 - ADD from, to
- from = {#* | D* | MEMORY}
- to = {D* | MEMORY}
- Set NZCV flags

0x11 - ADC from, to
- from = {#* | D* | MEMORY}
- to = {D* | MEMORY}
- supposed Carry flag

0x12 - SUB from, to
- from = {#* | D* | MEMORY}
- to = {D* | MEMORY}
- Set NZCV flags

0x13 - MUL from, to
- from = {#* | D* | MEMORY}
- to = {D* | MEMORY}
- Set NZCV flags

0x14 - DIV from, to
- from = {#* | D* | MEMORY}
- to = {D* | MEMORY}
- Set NZCV flags
- set C-flag if zero-dived

0x15 - REM from, to 
- from = {#* | D* | MEMORY}
- to = {D* | MEMORY}
- Set NZCV flags
- set C-flag if zero-dived
---
0x20 - AND from, to
- from = {#* | D* | MEMORY}
- to = {D* | MEMORY}

0x21 - OR from, to
- from = {#* | D* | MEMORY}
- to = {D* | MEMORY}

0x22 - XOR from, to
- from = {#* | D* | MEMORY}
- to = {D* | MEMORY}

0x23 - NOT from, to
- from = {#* | D* | MEMORY}
- to = {D* | MEMORY}
---
0x30 - LSL count, source
- source = {#* | D* | MEMORY}
- use C-flag

0x31 - LSR count, source
- source = {#* | D* | MEMORY}
- use C-flag

0x32 - ASL count, source
- source = {#* | D* | MEMORY}

0x33 - ASR count, source
- source = {#* | D* | MEMORY}
- sign saving
---
0x40 - JMP label
- PC <- label

0x41 - CALL label
- A7 <- A7 - 8
- (A7) <- PC
- PC <- label

0x42 - FUNC label, args_count, arg0, arg1, ..
- arg_i = {#* | D* | MEMORY}
- for i from 0 to count - 1
- - A7 <- A7 - 8
- - (A7) <- arg_i
- A7 <- A7 - 8
- (A7) <- PC
- PC <- label

0x43 - RET 
- PC <- (A7)
- A7 <- A7 + 8

0x44 - LINK A*, count_bytes
- A7 <- A7 - 8
- (A7) <- A*
- A* <- A7
- A7 <- A7 - count_bytes

0x45 - UNLK A*
- A7 <- A*
- A6 <- (A7)
- A7 <- A7 + 8
---

0x50 - BEQ label 

0x51 - BNE label

0x52 - BGT label

0x53 - BGE label

0x54 BLT label

0x55 - BLE label

0x56 - BCS label

0x57 - BCC label

0x58 - BVS label

0x59 - BVC label

---
0x60 - CMP that, with
- that, with = {#* | D* | MEMORY}
- set NZVC as for (that - with)

