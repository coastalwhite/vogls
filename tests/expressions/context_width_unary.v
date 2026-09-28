module tb;
  reg [4:0] a, b, c;
  initial begin
    a = (1'b0 - 2'b01);
    b = ~(1'b0 - 2'b01);
    c = -(1'b0 - 2'b01);

    $vogls_assert_eq(a, 31);
    $vogls_assert_eq(b, 0);
    $vogls_assert_eq(c, 1);
  end
endmodule
