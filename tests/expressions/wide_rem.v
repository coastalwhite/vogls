`timescale 1ns/1ps
module top;
  reg [99:0] a, r;
  initial begin
    a = 100'd5;
    r = a % 3;
	$vogls_assert_eq(r, 100'd2);
  end
endmodule
