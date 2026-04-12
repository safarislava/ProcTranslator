State Views
-
D0 - D7, A0 - A7, NZCV, PC

Instructions
-
4 bytes - word size

n operands instructions 
- 7 bits - operator code
- 1 bit - choice of size (0 - byte, 1 - long) 
- i byte - i operand

Branch instructions
- 7 bits - operator code
- 1 bit - reserved
- 7 bytes - address

Operand Description 
-
- [7:5] - mode
- [4:2] - main register 
- [1:0] - offset, support 32bit constant (00), D5 (01), D6 (10), D7 (11)

Mode:
-
- 0x0 - #* - direct (constant in next word)
- 0x1 - D* - data register
- 0x2 - A* - address register
- 0x3 - (A*) - indirect
- 0x4 - (A*)+ - indirect, post-increment
- 0x5 - -(A*) - indirect, pre-decrement
- 0x6 - (A*:O) - indirect, with offset
- 0x7 - (#*) - indirect, direct (constant in next word)

Operator code:
---
0x0 - HLT

---
0x01 - MOV.size from, to
- from = {#* | D* | A* | MEMORY}
- to = {D* | A* | MEMORY}

0x02 - MOVA.size from, to
- from = {D* | A*}
- to = {D* | A*}

0x03 - CMP.size that, with
- that, with = {#* | D* | A* | MEMORY}
- set NZVC as for (that - with)
---
0x10 - ADD.size from, to
- from = {#* | D* | A* | MEMORY}
- to = {D* | A* | MEMORY}
- Set NZCV flags

0x11 - ADC.size from, to
- from = {#* | D* | A* | MEMORY}
- to = {D* | A* | MEMORY}
- supposed Carry flag

0x12 - SUB.size from, to
- from = {#* | D* | A* | MEMORY}
- to = {D* | A* | MEMORY}
- Set NZCV flags

0x13 - MUL.size from, to
- from = {#* | D* | A* | MEMORY}
- to = {D* | A* | MEMORY}
- Set NZCV flags

0x14 - DIV.size from, to
- from = {#* | D* | A* | MEMORY}
- to = {D* | A* | MEMORY}
- Set NZCV flags
- set C-flag if zero-dived

0x15 - REM.size from, to 
- from = {#* | D* | A* | MEMORY}
- to = {D* | A* | MEMORY}
- Set NZCV flags
- set C-flag if zero-dived

0x16 - AND.size from, to
- from = {#* | D* | MEMORY}
- to = {D* | MEMORY}

0x17 - OR.size from, to
- from = {#* | D* | A* | MEMORY}
- to = {D* | A* | MEMORY}

0x18 - XOR.size from, to
- from = {#* | D* | A* | MEMORY}
- to = {D* | A* | MEMORY}

0x19 - NOT.size from, to
- from = {#* | D* | A* | MEMORY}
- to = {D* | A* | MEMORY}

0x1A - LSL.size count, dest
- count = {#* | D* | A* | MEMORY}
- dest = {#* | D* | A* | MEMORY}
- set C-flag

0x1B - LSR.size count, dest
- count = {#* | D* | A* | MEMORY}
- dest = {#* | D* | A* | MEMORY}
- set C-flag

0x1C - ASL.size count, dest
- count = {#* | D* | A* | MEMORY}
- dest = {#* | D* | A* | MEMORY}

0x1D - ASR.size count, dest
- count = {#* | D* | A* | MEMORY}
- dest = {#* | D* | A* | MEMORY}
- sign saving
---
0x20 - JMP label
- PC <- label

0x21 - CALL label
- PC <- label

0x22 - RET 
- PC <- (A7)
- A7 <- A7 + 8

0x23 - RET
- PC <- (A7)
- NZCV <- (A7)+
---
0x30 - BEQ label 

0x31 - BNE label

0x32 - BGT label

0x33 - BGE label

0x34 - BLT label

0x35 - BLE label

0x36 - BCS label

0x37 - BCC label

0x38 - BVS label

0x39 - BVC label

--- 
0x40 - VADD a, b
- a, b = {A*}

0x42 - VSUB a, b
- a, b = {A*}

0x43 - VMUL a, b
- a, b = {A*}

0x44 - VDIV a, b
- a, b = {A*}

0x45 - VREM a, b
- a, b = {A*}

0x46 - VAND a, b
- a, b = {A*}

0x47 - VOR a, b
- a, b = {A*}

0x48 - VXOR a, b
- a, b = {A*}

0x49 - VEND c
- c = {A*}
---
0x50 - IN port, dest
- dest = {D* | A* | MEMORY}

0x51 - OUT port, source
- source = {#* | D* | A* | MEMORY}

0x52 - EI 
- Enable interrupts

0x53 - DI
- Disable interrupts

0x60 - VCMPBEQ a, b
- a, b = {A*}
- return mask 

0x61 - VCMPBNE a, b
- a, b = {A*}
- return mask

0x62 - VCMPBGT a, b
- a, b = {A*}
- return mask

0x63 - VCMPBGE a, b
- a, b = {A*}
- return mask

0x64 - VCMPBLT a, b
- a, b = {A*}
- return mask

0x65 - VCMPBLE a, b
- a, b = {A*}
- return mask

0x66 - VCMPBCS a, b
- a, b = {A*}
- return mask

0x67 - VCMPBCC a, b
- a, b = {A*}
- return mask

0x68 - VCMPBVS a, b
- a, b = {A*}
- return mask

0x69 - VCMPBVC a, b
- a, b = {A*}
- return mask 

