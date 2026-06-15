module top;
  wire [31:0] a     = 32'h00000001;
  wire [31:0] b     = 32'h00000001;
  wire        valid = 1'b1;

  wire result    = |{a, b} & ~valid;
  wire result_ok = (|{a, b}) & ~valid;

  initial begin
    #1;
    $vogls_assert_eq(result, 1'b0);
    $vogls_assert_eq(result_ok, 1'b0);
  end
endmodule
