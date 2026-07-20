module tb;
   reg signed [31:0] a, b;
   reg signed [7:0]  a8, b8;
   integer           q;

   initial begin
      a =  100; b =  7; #1;
      $vogls_assert_eq(a / b,  14);
      $vogls_assert_eq(a % b,   2);
      a = -100; b =  7; #1;
      $vogls_assert_eq(a / b, -14);
      $vogls_assert_eq(a % b,  -2);
      a =  100; b = -7; #1;
      $vogls_assert_eq(a / b, -14);
      $vogls_assert_eq(a % b,   2);
      a = -100; b = -7; #1;
      $vogls_assert_eq(a / b,  14);
      $vogls_assert_eq(a % b,  -2);

      a =    0; b =  7; #1;
      $vogls_assert_eq(a / b,   0);
      $vogls_assert_eq(a % b,   0);
      a =  -1; b =  1; #1;
      $vogls_assert_eq(a / b,  -1);
      a = 2147483647; b = -1; #1;
      $vogls_assert_eq(a / b, -2147483647);
      a = -128; b = -1; #1;
      $vogls_assert_eq(a / b, 128);
      $vogls_assert_eq(a % b, 0);

      q = $signed(a) / $signed(b);
      $vogls_assert_eq(q, 128);
      $vogls_assert_eq(-100 / 7, -14);

      a8 = -128; b8 = -1; #1;
      $vogls_assert_eq(a8 / b8, 8'h80);
      a8 = -100; b8 = 7; #1;
      $vogls_assert_eq(a8 / b8, -14);
      $vogls_assert_eq(a8 % b8, -2);

`ifndef __VOGLS__TWO_VALUE_LOGIC
      a = 32'bx;         b = 7;      #1;
      $vogls_assert_eq(a / b, 32'bx);
      $vogls_assert_eq(a % b, 32'bx);
      a = 100;           b = 32'bx;  #1;
      $vogls_assert_eq(a / b, 32'bx);
      a = 32'hxxxx_0064; b = 7;      #1;
      $vogls_assert_eq(a / b, 32'bx);
      a = -100;          b = 0;      #1;
      $vogls_assert_eq(a / b, 32'bx);
      $vogls_assert_eq(a % b, 32'bx);
`endif
   end
endmodule
