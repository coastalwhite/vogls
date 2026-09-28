module tb;
  reg [7:0] a;
  initial begin
    a = 8'd3;
    #0 $vogls_assert_eq(8'd12 / a, 4);
  end
endmodule
