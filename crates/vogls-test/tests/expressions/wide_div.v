module top;
  reg [157:0] w;
  reg [15:0] q, m;
  initial begin
    w = 158'd100;
    q = w / 7;
    m = w % 7;
    $vogls_assert_eq(q, 14);
    $vogls_assert_eq(m, 2);
  end
endmodule
