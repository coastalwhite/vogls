// vogls: mode=four-value-logic
primitive udp_buf (o, i);
  output o;
  input  i;
  table
    0 : 0;
    1 : 1;
  endtable
endprimitive

primitive udp_and (o, i0, i1);
  output o;
  input  i0, i1;
  table
    0 ? : 0;
    ? 0 : 0;
    1 1 : 1;
  endtable
endprimitive

module tb;
  reg a;

  udp_buf b0 (p, a);
  udp_and a0 (q, a, p);
  udp_and a1 (r, p, 1'bz);

  initial begin
    a = 1'b0;
    #1;
    $vogls_assert_eq(p, 1'b0);
    $vogls_assert_eq(q, 1'b0);
    $vogls_assert_eq(r, 1'b0);
    a = 1'b1;
    #1;
    $vogls_assert_eq(p, 1'b1);
    $vogls_assert_eq(q, 1'b1);
    $vogls_assert_eq(r, 1'bx);
  end
endmodule
