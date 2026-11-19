// vogls: mode=two-value-logic
//
// `tran` switches that connect only some bits of a net. An undriven bit reads `0` in two-valued
// logic, so it must never be resolved against the drivers of a closure: `a[1]` and `a[2]` are
// only driven through their `tran`, while their neighbours keep their own drivers.
module tb;
   reg [3:0] x;
   reg       p, q, r;

   wire [3:0] a;
   wire       b, c, d;
   wire [1:0] s;

   assign a[1:0] = x[1:0];      // Bit 0 is in a closure, bit 1 is not.
   assign a[3]   = x[3];        // Bit 3 is in a closure, bit 2 is undriven through `a`.
   assign b      = p;
   assign c      = q;
   assign s[0]   = r;

   tran t0 (a[0], b);
   tran t1 (a[2], c);
   tran t2 (a[3], d);
   tran t3 (s[0], s[1]);        // Self-connected.

   initial begin
      x = 4'b1010; p = 1'b0; q = 1'b1; r = 1'b1;
      #1;
      $vogls_assert_eq(a, 4'b1110);
      $vogls_assert_eq({b, c, d}, 3'b011);
      $vogls_assert_eq(s, 2'b11);

      x = 4'b0111; p = 1'b1; q = 1'b0; r = 1'b0;
      #1;
      $vogls_assert_eq(a, 4'b0011);
      $vogls_assert_eq({b, c, d}, 3'b100);
      $vogls_assert_eq(s, 2'b00);

      x = 4'b1001; p = 1'b1; q = 1'b1; r = 1'b1;
      #1;
      $vogls_assert_eq(a, 4'b1101);
      $vogls_assert_eq({b, c, d}, 3'b111);
      $vogls_assert_eq(s, 2'b11);
   end
endmodule
