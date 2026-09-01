module tb;
   localparam GE_NEG_ZERO = (-1 >= 0);
   localparam GE_EQ       = ( 0 >= 0);
   localparam GE_POS_ZERO = ( 1 >= 0);
   localparam GE_DECR     = ((0 - 1) >= 0);

   localparam GT_POS_NEG  = ( 3 >  -1);
   localparam LE_NEG_ZERO = (-1 <= 0);
   localparam LE_POS_NEG  = ( 3 <= -1);

   initial begin
      $vogls_assert_eq(GE_NEG_ZERO, 1'b0);
      $vogls_assert_eq(GE_EQ,       1'b1);
      $vogls_assert_eq(GE_POS_ZERO, 1'b1);
      $vogls_assert_eq(GE_DECR,     1'b0);
      $vogls_assert_eq(GT_POS_NEG,  1'b1);
      $vogls_assert_eq(LE_NEG_ZERO, 1'b1);
      $vogls_assert_eq(LE_POS_NEG,  1'b0);
   end
endmodule
