// vogls: fail=lower
`timescale 1ns/1ps

module dut (output wire [3:0] o);

   wire [3:0] a = 4'd9;
   wire [3:0] b = 4'd1;

   // apbregs form: bare undeclared scalar on the LHS
   assign z = a[0] & b[0];

   // sub form: undeclared scalar inside an LHS concatenation
   assign {cout, o} = a - b;

endmodule

module tb;

   wire [3:0] o;

   dut u_dut (.o(o));

   initial begin
      #1;
      $display("o = %0d (expected 8)", o);
      $finish;
   end

endmodule
