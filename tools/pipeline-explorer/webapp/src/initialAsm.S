    .section .data
 
    .section .bss
    .align 4
primes:     .space 400000          # room for 100000 x 4-byte integers
 
    .section .text
    .global _start
 
_start:
    la   s0, primes          # s0 = &primes[0]
    li   t0, 2
    sw   t0, 0(s0)           # primes[0] = 2
    li   s1, 1               # s1 = count (number of primes found so far)
    li   s2, 3               # s2 = candidate (current number being tested)
    li   s3, 10              # s3 = target count
 
outer_loop:
    beq  s1, s3, found       # if count == 100000, we're done
 
    li   t1, 0
 
trial_loop:
    slli t2, t1, 2           # t2 = t1 * 4 (byte offset)
    add  t3, s0, t2          # t3 = &primes[t1]
    lw   t4, 0(t3)           # t4 = primes[t1]  (current trial divisor p)
 
    mul  t5, t4, t4          # t5 = p*p
    bgt  t5, s2, is_prime    # if p*p > candidate → candidate is prime
 
    rem  t6, s2, t4          # t6 = candidate % p
    beqz t6, not_prime       # remainder zero → candidate is composite
 
    addi t1, t1, 1
    blt  t1, s1, trial_loop  # while index < count, keep testing
 
is_prime:
    slli t2, s1, 2           # t2 = count * 4
    add  t3, s0, t2
    sw   s2, 0(t3)           # primes[count] = candidate
    addi s1, s1, 1           # count++
 
not_prime:
    addi s2, s2, 2           # candidate += 2
    j    outer_loop

found:
	ecall
