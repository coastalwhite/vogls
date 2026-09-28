module tb;
  reg [7:0] src;
 
  assign x = src;
  assign y = 1'b1;
  assign z = src[6], w = 1'b0;
 
  initial begin
    src = 8'hA5;
    #1;
    $vogls_assert_eq(x, 1'b1);
    $vogls_assert_eq(y, 1'b1);
    $vogls_assert_eq(z, 1'b0);
    $vogls_assert_eq(w, 1'b0);
    src = 8'hA4;
    #1;
    $vogls_assert_eq(x, 1'b0);
    $vogls_assert_eq(y, 1'b1);
    $vogls_assert_eq(z, 1'b0);
    $vogls_assert_eq(w, 1'b0);
  end
endmodule
