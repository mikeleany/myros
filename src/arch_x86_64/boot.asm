; asmsyntax=nasm
;
; Copyright 2020 Mike Leany
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
[extern main]

MBH_MAGIC       equ  0xE85250D6
MBH_LENGTH      equ  (_MB_HEADER.end - _MB_HEADER)
MBH_CHECKSUM    equ  -(MBH_MAGIC + MBH_LENGTH) & 0xFFFFFFFF


section .rodata align=16
_MB_HEADER:
    dd   MBH_MAGIC          ; magic
    dd   0                  ; architechture (i386 - protected mode)
    dd   MBH_LENGTH         ; header length
    dd   MBH_CHECKSUM       ; checksum

align  8
.flags_tag:
    dw   4                  ; type = 4 (flags tag)
    dw   0                  ; flags
    dd   12                 ; size
    dd   3                  ; console flags (EGA console)

align  8
.end_tag:
    dw   0                  ; type = 0 (end tag)
    dw   0                  ; flags
    dd   8                  ; size
.end:

SUCCESS_MSG:
    db "So far, so good!", 0
MULTIBOOT_ERR:
    db  "FATAL ERROR: Not loaded by Multiboot2 loader", 0
CPUID_ERR:
    db  "FATAL ERROR: CPUID instruction not supported", 0
LONG_MODE_ERR:
    db  "FATAL ERROR: Long mode not supported", 0

GDT_PRESENT     equ 1 << 47
GDT_CODE_SEG    equ (1 << 43) | (1 << 44)
GDT_LONG_ATTR   equ 1 << 53

align 16
GDT64:
.zero:
    dq 0
.code: equ $ - GDT64
    dq GDT_PRESENT | GDT_CODE_SEG | GDT_LONG_ATTR
.pointer:
    dw $ - GDT64 - 1
    dq GDT64
.end:


PTF_PRESENT     equ  1 << 0
PTF_WRITABLE    equ  1 << 1
PTF_LARGE_PAGE  equ  1 << 7

section .data
align 0x1000
L2_TABLE_ID_MAP: ; identity map the first 16 MiB just above zero
    dq 0x000000 | PTF_PRESENT | PTF_WRITABLE | PTF_LARGE_PAGE
    dq 0x200000 | PTF_PRESENT | PTF_WRITABLE | PTF_LARGE_PAGE
    dq 0x400000 | PTF_PRESENT | PTF_WRITABLE | PTF_LARGE_PAGE
    dq 0x600000 | PTF_PRESENT | PTF_WRITABLE | PTF_LARGE_PAGE
    dq 0x800000 | PTF_PRESENT | PTF_WRITABLE | PTF_LARGE_PAGE
    dq 0xa00000 | PTF_PRESENT | PTF_WRITABLE | PTF_LARGE_PAGE
    dq 0xc00000 | PTF_PRESENT | PTF_WRITABLE | PTF_LARGE_PAGE
    dq 0xe00000 | PTF_PRESENT | PTF_WRITABLE | PTF_LARGE_PAGE
    times 504 dq 0

align 0x1000
L3_TABLE_ID_MAP:
    dq L2_TABLE_ID_MAP + PTF_PRESENT + PTF_WRITABLE
    times 511 dq 0

align 0x1000
L1_TABLE_STACK: ; map the 16 KiB just below 0
    times 508 dq 0
    dq STACK + 0x0000  + PTF_PRESENT + PTF_WRITABLE
    dq STACK + 0x1000  + PTF_PRESENT + PTF_WRITABLE
    dq STACK + 0x2000  + PTF_PRESENT + PTF_WRITABLE
    dq STACK + 0x3000  + PTF_PRESENT + PTF_WRITABLE

align 0x1000
L2_TABLE_STACK:
    times 511 dq 0
    dq L1_TABLE_STACK  + PTF_PRESENT + PTF_WRITABLE

align 0x1000
L3_TABLE_STACK:
    times 511 dq 0
    dq L2_TABLE_STACK  + PTF_PRESENT + PTF_WRITABLE

align 0x1000
L4_TABLE:
    dq L3_TABLE_ID_MAP + PTF_PRESENT + PTF_WRITABLE
    times 510 dq 0
    dq L3_TABLE_STACK  + PTF_PRESENT + PTF_WRITABLE


[BITS 32]
section .text
[global _start]
_start:
    ; setup the stack
    lea esp, [STACK.end]

    ; verify multiboot magic number
    cmp eax, 0x36d76289
    jne error.multiboot

    ; save multiboot info address
    mov [MB_INFO_ADDR], ebx

    ; verify cpuid instruction is supported
    pushfd                      ; push eflags to the stack
    mov eax, [esp]              ; store the original eflags
    xor dword [esp], 1 << 21    ; invert ID flag (bit 21)
    popfd                       ; load new eflags
    pushfd                      ; push new eflags to see if it changed
    xor eax, [esp]              ; get changes in eflags
    test eax, 1 << 21           ; test if the ID flag was successfully changed
    jz error.cpuid

    ; verify long mode is supported
    mov eax, 0x80000000         ; check for highest available extended function
    cpuid
    cmp eax, 0x80000001         ; make sure extended feature flags funtion is available
    jb error.long_mode
    mov eax, 0x80000001         ; check extended feature flags
    cpuid
    test edx, 1 << 29           ; test if the LM flag (bit 29) is set
    jz error.long_mode

    ; set the PAE bit in cr4
    mov eax, cr4
    or eax, 1 << 5
    mov cr4, eax

    ; load page tables
    lea eax, [L4_TABLE]
    mov cr3, eax

    ; enable long mode
    mov ecx, 0xC0000080
    rdmsr
    or eax, 1 << 8
    wrmsr

    ; enable paging
    mov eax, cr0
    or eax, 1 << 31
    mov cr0, eax

    ; load the 64-bit GDT
    lgdt [GDT64.pointer]
    ; jump to enter long mode
    jmp GDT64.code:long_mode_start
.end:

error:
.multiboot:
    lea edi, [MULTIBOOT_ERR]
    jmp print_and_halt
.cpuid:
    lea edi, [CPUID_ERR]
    jmp print_and_halt
.long_mode:
    lea edi, [LONG_MODE_ERR]
    jmp print_and_halt
.end:

print_and_halt:
    lea edx, [0xb8000]      ; video memory base address
    mov ecx, 0              ; byte offset
    mov ah, 0x0f            ; color - white on black
.loop:
    mov al, [edi+ecx]       ; read the next character
    cmp al, 0               ; if it's null
    jz .halt                ; break
    mov [edx+ecx*2], ax     ; write the character (with the color) to the screen
    inc ecx                 ; increment the byte offset
    jmp .loop
.halt:
    hlt
    jmp .halt
.end:


[bits 64]
section .text
long_mode_start:
    ; load 0 into all data segment registers
    mov ax, 0
    mov ss, ax
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

    ; set stack to grow down from 0
    mov rsp, 0

    call main
.halt:
    hlt
    jmp .halt
.end:


section .bss
align 0x1000
STACK:
    resb 0x4000
.end:           ; stack grows down from here

MB_INFO_ADDR:
    resd 1
