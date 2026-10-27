// vogls: mode=four-value-logic
// vogls: verify-stdout
`timescale 1ns/10ps
module m(A, S);
  input A;
  output S;
  assign S = A;
  specify
    (A => S) = (1.5, 2.25);
  endspecify
endmodule

module tb();
  reg a;
  wire s;
  m dut(a, s);
  always @(s) $display("[T=%0.2f] s = %b", $realtime, s);
  initial begin
    a = 0;
    #10 a = 1;
    #10 a = 0;
  end
endmodule
