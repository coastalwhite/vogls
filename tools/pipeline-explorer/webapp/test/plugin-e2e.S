# Test program for the design-plugin end-to-end check.
#
# Deliberately touches no memory: the explorer's testbenches only wire up a
# 1 KiB array, and `initialAsm.S` traps on its first store because its `.bss`
# starts at the end of it. This one just spins in registers, long enough to
# fill a few hundred cycles.
#
# It leans on multiply and remainder so the Hazard3 knobs show up in the
# trace: without the M extension these are illegal instructions, and the
# single-cycle multiply and muldiv-unroll options change how long each one
# occupies the execute stage.

    .section .text
    .global _start

_start:
    li   s0, 0               # s0 = accumulator
    li   s1, 1               # s1 = i
    li   s2, 12              # s2 = limit

loop:
    mul  t0, s1, s1          # t0 = i*i
    add  s0, s0, t0
    rem  t1, s0, s1          # t1 = acc % i
    add  s0, s0, t1
    addi s1, s1, 1
    blt  s1, s2, loop

    ecall
