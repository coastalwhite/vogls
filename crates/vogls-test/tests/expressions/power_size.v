module top;
  reg [3:0] g;
  initial begin
    g = 4'd11;
    $vogls_assert_eq(g ** 2, 9);
  end
endmodule
