// vogls: verify-stdout
`timescale 1ns/1ps
module top;
  reg [99:0] a;
  initial begin
    a = 100'd5;
    $display("%0d", a);
  end
endmodule
