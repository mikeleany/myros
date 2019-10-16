; asmsyntax=nasm
;
; Copyright 2019 Mike Leany
;
; Licensed under the Apache License, Version 2.0 (the "License");
; you may not use this file except in compliance with the License.
; You may obtain a copy of the License at
;
;     <http://www.apache.org/licenses/LICENSE-2.0>
;
; Unless required by applicable law or agreed to in writing, software
; distributed under the License is distributed on an "AS IS" BASIS,
; WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
; See the License for the specific language governing permissions and
; limitations under the License.

section .rodata align=16
MSG:
    db  "Hello, world!", 0

[BITS 32]
section .text
[global _start]
_start:
    lea edx, [0xb8000]      ; video memory base address
    lea ebx, [MSG]          ; our message
    mov ecx, 0              ; byte offset
    mov ah, 0x0f            ; color - white on black
.loop:
    mov al, [ebx+ecx]       ; read the next character
    cmp al, 0               ; if it's null
    jz .loop_end            ; break
    mov [edx+ecx*2], ax     ; write the character (with the color) to the screen
    inc ecx                 ; increment the byte offset
    jmp .loop
.loop_end:

.halt:
    hlt
    jmp .halt
.end:
