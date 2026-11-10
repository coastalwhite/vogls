# Sections placed relative to each other rather than at fixed addresses.
#
# .data follows .text rounded up to 16, .rodata follows .data rounded up to 4,
# and .bss follows .data too - so .rodata and .bss deliberately share a base.
# trva-text: 0x8000_0000
# trva-data: after:text:align=16
# trva-rodata: after:data:align=4
# trva-bss: after:data:gap=8:align=4

.section .text
    li   a0, 1
    li   a1, 2
    add  a2, a0, a1
    ecall

.section .data
    .space 5

.section .rodata
    .space 3

.section .bss
    .align 4
    .space 64
