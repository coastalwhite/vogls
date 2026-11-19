// vogls: mode=four-value-logic
//
// Delayed continuous assignments as drivers of nets: resolved with undelayed drivers, into a
// concatenation, sign-extended and chained.
//
// The expected values match Icarus Verilog.
module tb;
   reg        [3:0] a, b;
   reg signed [3:0] sa;

   // A delayed and an undelayed driver. The delayed one is `x` until it first drives.
   wire [3:0] r;
   assign #10 r = a;
   assign r = b;

   // A delayed expression into a concatenation keeps its carry.
   wire [1:0] lo;
   wire [2:0] hi;
   assign #10 {hi, lo} = a + b;

   // Halves of a net with different delays.
   wire [3:0] p;
   assign #10 p[1:0] = a[1:0];
   assign #20 p[3:2] = b[3:2];

   // A delayed sign-extension.
   wire [7:0] sx;
   assign #10 sx = sa;

   // Chained delays.
   wire [3:0] ch1, ch2;
   assign #10 ch1 = a;
   assign #10 ch2 = ch1;

   // A delayed constant.
   wire [3:0] k;
   assign #10 k = 4'b1100;

   initial begin
      a = 4'h0; b = 4'h0; sa = -4'sd1;
      #5;
      $vogls_assert_eq(r, 4'bxxxx);
      $vogls_assert_eq({hi, lo}, 5'bxxxxx);
      $vogls_assert_eq(k, 4'bxxxx);
      #10;
      $vogls_assert_eq(r, 4'b0000);
      $vogls_assert_eq({hi, lo}, 5'b00000);
      $vogls_assert_eq(p, 4'bxx00);
      $vogls_assert_eq(sx, 8'b11111111);
      $vogls_assert_eq(ch2, 4'bxxxx);
      $vogls_assert_eq(k, 4'b1100);
      #10;
      $vogls_assert_eq(ch2, 4'b0000);

      a = 4'hf; b = 4'h3; sa = 4'sd3;
      // The undelayed driver changes first, then both drive and conflict.
      #5;
      $vogls_assert_eq(r, 4'b00xx);
      $vogls_assert_eq({hi, lo}, 5'b00000);
      #10;
      $vogls_assert_eq(r, 4'bxx11);
      $vogls_assert_eq({hi, lo}, 5'b10010);
      $vogls_assert_eq(p, 4'b0011);
      $vogls_assert_eq(sx, 8'b00000011);
      $vogls_assert_eq(ch1, 4'b1111);
      $vogls_assert_eq(ch2, 4'b0000);
      #10;
      $vogls_assert_eq(p, 4'b0011);
      $vogls_assert_eq(ch2, 4'b1111);
   end
endmodule
