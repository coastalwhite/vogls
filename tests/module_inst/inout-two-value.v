// vogls: mode=two-value-logic
module driver(inout [3:0] p, input [3:0] d);
   assign p = d;
endmodule

module reader(inout [3:0] p, output [3:0] o);
   assign o = p;
endmodule

module tb;
   // Driven from the inside.
   reg  [3:0] d;
   wire [3:0] a;
   driver u0 (a, d);

   // Driven from the outside.
   reg  [3:0] e;
   wire [3:0] b, o;
   assign b = e;
   reader u1 (b, o);

   initial begin
      d = 4'b1010;
      e = 4'b0110;
      #1;
      $vogls_assert_eq(4'b1010, a);
      $vogls_assert_eq(4'b0110, o);
      d = 4'b0101;
      e = 4'b1001;
      #1;
      $vogls_assert_eq(4'b0101, a);
      $vogls_assert_eq(4'b1001, o);
   end
endmodule
