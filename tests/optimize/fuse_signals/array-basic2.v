module top;
  wire [7:0] src [0:1];
  assign src[0] = 8'hAA;
  assign src[1] = 8'hBB;

  reg idx = 0;
  wire [7:0] val = src[idx];

  initial begin
    #1;
    idx=0; #1; $vogls_assert_eq(val, 8'hAA);
    idx=1; #1; $vogls_assert_eq(val, 8'hBB);
  end
endmodule
