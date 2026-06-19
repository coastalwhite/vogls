1:
    beq  a0, a1, 2f
    bne  a0, a1, 2f
    blt  a0, a1, 2f
    bgt  a0, a1, 2f
    beqz a0, 2f
    bnez a0, 2f
2:
    beq  a0, a1, 1b
    bne  a0, a1, 1b
