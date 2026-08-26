// vogls: fail=elaborate
`timescale 1ns/1ps
module tb();
    reg a, b;
    wire z;
    bufif0 (z, a, b);
endmodule
