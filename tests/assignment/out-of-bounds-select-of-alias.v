// vogls: mode=four-value-logic
//
// A select with an index that is not constant reads `x` for the bits beyond its signal, also when
// the signal is aliased to a slice of a wider signal. The neighbouring bits of the wider signal
// must not leak into the select.
//
// The expected values match Icarus Verilog.
module tb;
   reg  [1:0] i;

   // A net aliased to the low bits of a variable.
   reg  [7:0] b;
   wire [3:0] n = b[3:0];
   wire [1:0] sn = n[i +: 2];

   // A variable that drives the low bits of a net, next to constant bits.
   reg  [3:0] v;
   wire [7:0] w;
   assign w[3:0] = v;
   assign w[7:4] = 4'b1111;
   wire [1:0] sv = v[i +: 2];

   // The same, with a parameter in a concatenation.
   localparam [3:0] P = 4'b1001;
   reg  [3:0] a;
   wire [7:0] wc = {P, a};
   wire [1:0] sa = a[i +: 2];

   initial begin
      b = 8'b1111_0101; v = 4'b0101; a = 4'b0101; i = 2'd1;
      #1;
      $vogls_assert_eq(sn, 2'b10);
      $vogls_assert_eq(sv, 2'b10);
      $vogls_assert_eq(sa, 2'b10);

      i = 2'd3;
      #1;
      $vogls_assert_eq(sn, 2'bx0);
      $vogls_assert_eq(sv, 2'bx0);
      $vogls_assert_eq(sa, 2'bx0);
   end
endmodule
