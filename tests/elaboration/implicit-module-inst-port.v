// vogls: mode=four-value-logic
module inv (o, i);
  output o;
  input  i;
  assign o = ~i;
endmodule

module or2 (o, i0, i1);
  output o;
  input  i0, i1;
  assign o = i0 | i1;
endmodule

module wide (o);
  output [7:0] o;
  assign o = 8'hA5;
endmodule

module tb;
  reg a;

  inv  m0 (p, a);
  inv  m1 (.o(q), .i(p));
  or2  m2 (r, a, 1'bz);
  wide m3 (v);

  initial begin
    a = 1'b0;
    #1;
    $vogls_assert_eq(p, 1'b1);
    $vogls_assert_eq(q, 1'b0);
    $vogls_assert_eq(r, 1'bx);
    $vogls_assert_eq(v, 1'b1);
    a = 1'b1;
    #1;
    $vogls_assert_eq(p, 1'b0);
    $vogls_assert_eq(q, 1'b1);
    $vogls_assert_eq(r, 1'b1);
    $vogls_assert_eq(v, 1'b1);
  end
endmodule
