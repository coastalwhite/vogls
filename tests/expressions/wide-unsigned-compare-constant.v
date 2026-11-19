module top;
  reg [99:0] a;
  initial begin
    a = 100'd5;
    #1;
    $vogls_assert_eq(a >= 100'd3, 1'b1);
    $vogls_assert_eq(a >= 100'd7, 1'b0);
    $vogls_assert_eq(a < 100'd3, 1'b0);
    $vogls_assert_eq(a > 100'd7, 1'b0);
    $vogls_assert_eq(a >= 100'd5, 1'b1);
    $vogls_assert_eq(a <= 100'd5, 1'b1);
  end
endmodule
