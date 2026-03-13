State Views
-
D0 - D7, A0 - A7, NZCV, PC

Instructions
-
- 7 bits - operator code
- 1 bit - choice of size (0 - byte, 1 - long) 
- next n bytes - operand's descriptions, 
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
- 0x7 - (#*) - indirect, direct

Operator code:
---
0x0 - HLT

---
0x01 - MOV.size from, to
- from = {#* | D* | MEMORY}
- to = {D* | MEMORY}

0x02 - MOVA.size from, to
- from = {D* | A*}
- to = {D* | A*}
---
0x10 - ADD.size from, to
- from = {#* | D* | MEMORY}
- to = {D* | MEMORY}
- Set NZCV flags

0x11 - ADC.size from, to
- from = {#* | D* | MEMORY}
- to = {D* | MEMORY}
- supposed Carry flag

0x12 - SUB.size from, to
- from = {#* | D* | MEMORY}
- to = {D* | MEMORY}
- Set NZCV flags

0x13 - MUL.size from, to
- from = {#* | D* | MEMORY}
- to = {D* | MEMORY}
- Set NZCV flags

0x14 - DIV.size from, to
- from = {#* | D* | MEMORY}
- to = {D* | MEMORY}
- Set NZCV flags
- set C-flag if zero-dived

0x15 - REM.size from, to 
- from = {#* | D* | MEMORY}
- to = {D* | MEMORY}
- Set NZCV flags
- set C-flag if zero-dived
---
0x20 - AND.size from, to
- from = {#* | D* | MEMORY}
- to = {D* | MEMORY}

0x21 - OR.size from, to
- from = {#* | D* | MEMORY}
- to = {D* | MEMORY}

0x22 - XOR.size from, to
- from = {#* | D* | MEMORY}
- to = {D* | MEMORY}

0x23 - NOT.size from, to
- from = {#* | D* | MEMORY}
- to = {D* | MEMORY}
---
0x30 - LSL.size count, source
- source = {#* | D* | MEMORY}
- use C-flag

0x31 - LSR.size count, source
- source = {#* | D* | MEMORY}
- use C-flag

0x32 - ASL.size count, source
- source = {#* | D* | MEMORY}

0x33 - ASR.size count, source
- source = {#* | D* | MEMORY}
- sign saving
---
0x40 - JMP label
- PC <- label

0x41 - CALL label
- A7 <- A7 - 8
- (A7) <- PC
- PC <- label

0x42 - RET 
- PC <- (A7)
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
0x60 - CMP.size that, with
- that, with = {#* | D* | MEMORY}
- set NZVC as for (that - with)

