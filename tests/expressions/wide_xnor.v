`timescale 1ns/1ps
module top;
  reg [99:0] a, b, r;
  initial begin
    a = 100'h78; b = 100'h07;
    #1 r = a ~^ b;
    #1 $vogls_assert_eq(r[7:0], 8'h80);
  end
endmodule
