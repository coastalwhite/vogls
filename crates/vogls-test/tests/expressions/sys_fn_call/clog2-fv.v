module top;
  reg [15:0] a;
  reg [15:0] r;
  initial begin
    a = 16'd5;
    r = $clog2(a);
    $vogls_assert_eq(r, 3);
  end
endmodule
