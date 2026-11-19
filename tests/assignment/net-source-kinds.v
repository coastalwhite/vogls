// The kinds of sources of a net assignment that are not plain wiring: parameters, expressions that
// keep a carry or sign in the context of the destination, selects with an index that is not
// constant and reals.
//
// The expected values match Icarus Verilog.
module tb;
   reg        [3:0] a, b;
   reg signed [3:0] sa;
   reg        [1:0] i;

   // The context of the destination keeps the carry.
   wire [4:0] sum = a + b;
   wire [4:0] sum2;
   assign sum2 = a + b;

   // Parameters, signed, unsigned and selected.
   localparam        [3:0] P  = 4'b1001;
   localparam signed [3:0] SP = -3;
   wire [7:0] wp  = P;
   wire [7:0] wsp = SP;
   wire [1:0] wp2 = P[2:1];
   wire [7:0] wcat = {P, a};

   // Selects with an index that is not constant.
   wire       sel  = a[i];
   wire [1:0] sel2 = a[i +: 2];

   // A signed expression is sign-extended, a concatenation is not.
   wire [7:0] neg  = -sa;
   wire [7:0] zx   = {sa};
   wire [7:0] mul  = sa * 2'sb11;

   // A concatenation as destination is unsigned, but a signed source is still sign-extended.
   wire [1:0] lo;
   wire [5:0] hi;
   assign {hi, lo} = sa;

   // A real is converted.
   wire [7:0] re = 3.7;

   initial begin
      a = 4'hf; b = 4'h3; sa = -4'sd3; i = 2'd1;
      #1;
      $vogls_assert_eq(sum, 5'b10010);
      $vogls_assert_eq(sum2, 5'b10010);
      $vogls_assert_eq(wp, 8'b00001001);
      $vogls_assert_eq(wsp, 8'b11111101);
      $vogls_assert_eq(wp2, 2'b00);
      $vogls_assert_eq(wcat, 8'b10011111);
      $vogls_assert_eq(sel, 1'b1);
      $vogls_assert_eq(sel2, 2'b11);
      $vogls_assert_eq(neg, 8'b00000011);
      $vogls_assert_eq(zx, 8'b00001101);
      $vogls_assert_eq(mul, 8'b00000011);
      $vogls_assert_eq({hi, lo}, 8'b11111101);
      $vogls_assert_eq(re, 8'd4);

      a = 4'b0101; i = 2'd3; sa = 4'sd5;
      #1;
      $vogls_assert_eq(sum, 5'b01000);
      $vogls_assert_eq(wcat, 8'b10010101);
      $vogls_assert_eq(sel, 1'b0);
      $vogls_assert_eq(sel2[0], 1'b0);
      $vogls_assert_eq(neg, 8'b11111011);
      $vogls_assert_eq(zx, 8'b00000101);
      $vogls_assert_eq(mul, 8'b11111011);
      $vogls_assert_eq({hi, lo}, 8'b00000101);
   end
endmodule
