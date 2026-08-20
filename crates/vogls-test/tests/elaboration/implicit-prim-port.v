// vogls: mode=four-value-logic
module tb;
  reg a;

  buf b0 (p, a);
  not n0 (q, p);
  and a0 (r, a, p);
  or  o0 (s, q, 1'bz);

  initial begin
    a = 1'b0;
    #1;
    $vogls_assert_eq(p, 1'b0);
    $vogls_assert_eq(q, 1'b1);
    $vogls_assert_eq(r, 1'b0);
    $vogls_assert_eq(s, 1'b1);
    a = 1'b1;
    #1;
    $vogls_assert_eq(p, 1'b1);
    $vogls_assert_eq(q, 1'b0);
    $vogls_assert_eq(r, 1'b1);
    $vogls_assert_eq(s, 1'bx);
  end
endmodule
