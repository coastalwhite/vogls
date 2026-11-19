// vogls: mode=four-value-logic
//
// Bits of a net that no driver covers stay `z`, also when a variable drives the other bits of the
// net directly and carries its initial value over into them.
//
// The expected values match Icarus Verilog.
module tb;
   // Two variables drive disjoint halves of a net. One has an initial value.
   reg  [3:0] b = 4'd5;
   reg  [3:0] c;
   wire [7:0] a;
   assign a[3:0] = b;
   assign a[7:4] = c;

   // A variable drives the low bits, the top bits are undriven.
   reg  [3:0] r = 4'b1010;
   wire [5:0] w;
   assign w[3:0] = r;

   // A variable without an initial value drives one bit, the other is undriven.
   reg        x;
   wire [1:0] wx;
   assign wx[0] = x;

   // Non-blocking assigned variables drive the halves of a net.
   reg        clk = 1'b0;
   reg  [1:0] n1, n2;
   wire [3:0] nb;
   assign nb = {n2, n1};
   always @(posedge clk) begin
      n1 <= n1 + 2'd1;
      n2 <= n1;
   end

   initial begin
      n1 = 2'd0;
      n2 = 2'd3;
      #1;
      $vogls_assert_eq(a, 8'bxxxx0101);
      $vogls_assert_eq(w, 6'bzz1010);
      $vogls_assert_eq(wx, 2'bzx);
      $vogls_assert_eq(nb, 4'b1100);

      c = 4'hc;
      clk = 1'b1;
      #1;
      $vogls_assert_eq(a, 8'b11000101);
      $vogls_assert_eq(nb, 4'b0001);

      b = 4'd2;
      x = 1'b1;
      r = 4'b0000;
      clk = 1'b0;
      #1;
      $vogls_assert_eq(a, 8'b11000010);
      $vogls_assert_eq(w, 6'bzz0000);
      $vogls_assert_eq(wx, 2'bz1);

      clk = 1'b1;
      #1;
      $vogls_assert_eq(nb, 4'b0110);
   end
endmodule
