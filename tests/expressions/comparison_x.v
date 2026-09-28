module top;
  reg [23:0] p0;
  initial begin
    p0 = 'd3;
    $vogls_assert_eq(((p0 / (&(24'h5ccee6 ** 2))) >= (((1'h0 >> 1'h1) ? (-p0) : (24'h82686b < 24'hc3a4da)) != (~(p0 != p0)))), 1'bx);
  end
endmodule
