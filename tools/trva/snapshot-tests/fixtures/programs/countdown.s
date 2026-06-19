    li   a0, 10
loop:
    addi a0, a0, 2047
    bnez a0, loop
    ecall
