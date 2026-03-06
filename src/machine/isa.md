State Views
-
D0 - D7, A0 - A7, NZVC, PC
A7 - stack pointer 

Addressation:
-
- #* - direct
- $A* - absolute
- A* - relational
- (A*) - relational, indirect
- (A*)+ - relational, indirect, post-increment
- -(A*) - relational, indirect, pre-decrement
- (A*:D*) - relational, indirect, with offset


Instructions:
-
MOV from, to
- from = {#* | D* | MEMORY}
- to = {D* | MEMORY}

MOVA from, to
- from = {D* | A*}
- to = {D* | A*}
---
ADD, SUB, MUL, DIV, REM from, to 
- from = {#* | D* | MEMORY}
- to = {D* | MEMORY}
---
AND, OR, XOR, NOT from, to
- from = {#* | D* | MEMORY}
- to = {D* | MEMORY}
---
LSL source, D*
- source = {#* | D* | MEMORY}
- use C-flag

LSR source, D*
- source = {#* | D* | MEMORY}
- use C-flag

ASL source, D*
- source = {#* | D* | MEMORY}

ASR source, D*
- source = {#* | D* | MEMORY}
- sign saving
---
JMP label
- PC <- label

CALL label
- A7 <- A7 - 8
- (A7) <- PC
- PC <- label

FUNC label, args_count, arg0, arg1, ..
- for i from 0 to count - 1
- - A7 <- A7 - 8
- - (A7) <- arg_i
- A7 <- A7 - 8
- (A7) <- PC
- PC <- label

RET 
- PC <- (A7)
- A7 <- A7 + 8

LINK A*, count_bytes
- A7 <- A7 - 8
- (A7) <- A*
- A* <- A7
- A7 <- A7 - count_bytes

UNLK A*
- A7 <- A*
- A6 <- (A7)
- A7 <- A7 + 8
---
CMP that, with
- that, with = {#* | D* | MEMORY}
- set NZVC as for (that - with)

BEQ, BNE, BGT, BGE, BLT, BLE
BCS, BCC, BVS, BVC