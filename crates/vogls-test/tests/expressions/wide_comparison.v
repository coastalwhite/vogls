module top;
  reg signed [64:0] a, b;
  reg lt, le, gt, ge;
  initial begin
    a = 65'sd1;
    b = -65'sd1;
    lt = a < b;
    le = a <= b;
    gt = a > b;
    ge = a >= b;
    $vogls_assert_eq(lt, 0);
    $vogls_assert_eq(le, 0);
    $vogls_assert_eq(gt, 1);
    $vogls_assert_eq(ge, 1);
  end
endmodule
