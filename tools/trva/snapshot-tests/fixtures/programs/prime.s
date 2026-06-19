    .section .data
 
    .section .bss
    .align 4
primes:     .space 400
 
    .section .text
    .global _start
 
_start:
    la   s0, primes
    li   t0, 2
    sw   t0, 0(s0)
    li   s1, 1
    li   s2, 3
    li   s3, 100
 
outer_loop:
    beq  s1, s3, found
 
    li   t1, 0
 
trial_loop:
    slli t2, t1, 2
    add  t3, s0, t2
    lw   t4, 0(t3)
 
    mul  t5, t4, t4
    bgt  t5, s2, is_prime
 
    rem  t6, s2, t4
    beqz t6, not_prime
 
    addi t1, t1, 1
    blt  t1, s1, trial_loop
 
is_prime:
    slli t2, s1, 2
    add  t3, s0, t2
    sw   s2, 0(t3)
    addi s1, s1, 1
 
not_prime:
    addi s2, s2, 2
    j    outer_loop
 
found:
    slli x0, x0, 31
	ebreak
    srai x0, x0, 7
