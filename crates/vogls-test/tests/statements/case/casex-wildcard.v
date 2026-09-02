module top;
  reg [2:0] s;
  reg [7:0] r;
  initial begin
    s = 3'bxxx;
    casex (s)
      3'h0: r = 8'd3;
      default: r = 8'd9;
    endcase
    $vogls_assert_eq(r, 8'd3);
  end
endmodule
