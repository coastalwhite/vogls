// vogls: mode=two-value-logic
// vogls: backend=bytecode
// vogls: disable-optimization=*
// vogls: panic
`timescale 1ns/1ps
module tb;
   reg [511:0] r;
   reg [31:0]  v;
   initial r = {v[31:0], 480'h0};
endmodule
