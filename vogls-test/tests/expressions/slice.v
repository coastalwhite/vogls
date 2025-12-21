module x();
	wire [32:0] a = 33'h0204_0608;
    initial begin
        $vogls_assert_eq(a[7:0], 8'h08);
        $vogls_assert_eq(a[11:0], 12'h608);
        $vogls_assert_eq(a[31:0], 32'h0204_0608);
        $vogls_assert_eq(a[31:4], 28'h0204_060);
        $vogls_assert_eq(a[28:1], 28'h0102_0304);
        $vogls_assert_eq(a[30:3], 28'h0040_80C1);
    end
endmodule
