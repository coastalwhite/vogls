`timescale 1ns/1ps
module tb;
   reg [511:0] r;
   reg [31:0]  v;
   initial begin
      v = 5;
      r = {v[31:0], 480'h0};
	  $vogls_assert_eq(r[15:0], 16'h0000);
   end
endmodule
