module top;
  reg signed [99:0] sa, sb, sc;
  reg signed [127:0] la, lb;
  initial begin
    sa = 0; sb = 100'h1_0000_0000_0000_0000; sc = -100'sd5;
    la = 128'h8000_0000_0000_0000_0000_0000_0000_0000; lb = 1;
    #1;
    $vogls_assert_eq(sa < sb, 1'b1);
    $vogls_assert_eq(sa <= sb, 1'b1);
    $vogls_assert_eq(sa > sb, 1'b0);
    $vogls_assert_eq(sb > sa, 1'b1);

    $vogls_assert_eq(sc < sa, 1'b1);
    $vogls_assert_eq(sc < sb, 1'b1);
    $vogls_assert_eq(sb < sc, 1'b0);
    $vogls_assert_eq(sc >= sc, 1'b1);

    $vogls_assert_eq(la < lb, 1'b1);
    $vogls_assert_eq(lb < la, 1'b0);
    $vogls_assert_eq(la <= la, 1'b1);
  end
endmodule
