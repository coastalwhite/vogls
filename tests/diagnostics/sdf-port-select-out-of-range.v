// vogls: verify-diagnostics
// vogls: annotate-sdf
// vogls: backend=bytecode
// vogls: mode=two-value-logic
// vogls: fail=lower
`timescale 1ns/1ns
module buffer(input a, output b);
    assign b = a;
endmodule
module top();
    reg [3:0] in;
    wire b;

    buffer u(.a(in[1]), .b(b));
endmodule
