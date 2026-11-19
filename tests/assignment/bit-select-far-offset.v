// vogls: mode=four-value-logic
module top;
  reg [9999:0] w;
  initial begin
    w = 0;
    w[4095] = 1'b1;
    w[4096] = 1'b1;
    w[5000] = 1'bx;
    w[9999] = 1'bz;
    #1;
    $vogls_assert_eq(w[4095], 1'b1);
    $vogls_assert_eq(w[4096], 1'b1);
    $vogls_assert_eq(w[5000], 1'bx);
    $vogls_assert_eq(w[9999], 1'bz);
    $vogls_assert_eq(w[8192], 1'b0);
    $vogls_assert_eq(w[4099:4092], 8'h18);
  end
endmodule
