// vogls: verify-ir
module top;
  wire [1:0] src [0:1];  // 2 elements, both driven
  assign src[0] = 2'b10;
  assign src[1] = 2'b00;

  localparam I = 0;
  wire w; assign w = src[I][1];

`ifndef __VOGLS_VERIFY_IR
  initial #0 $vogls_assert_eq(w, 1);
`endif
endmodule
