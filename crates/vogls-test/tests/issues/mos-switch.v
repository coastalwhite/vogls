// vogls: fail=elaborate
`timescale 1ns/1ps
module tb();
    reg a, b;
    wire z;
    pmos (z, a, b);
endmodule
