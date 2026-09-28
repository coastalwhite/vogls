module top;
  reg [3:0] g, x;
  initial begin
    g = 4'd11;
    x = g ** 2;
    $vogls_assert_eq(x, 9);
  end
endmodule
