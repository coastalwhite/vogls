module top();
    reg [7:0] a = 8'b0;
    initial #0 a = {1'd1, a[7:1]};
    initial #1 $vogls_assert_eq(a, 8'h80);
endmodule
