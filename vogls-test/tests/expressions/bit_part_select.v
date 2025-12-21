module x();
	wire [1:0] a2 = 2'b01;
	wire [63:0] a64 = { 8'hFF, 56'h0 };
    initial begin
        $vogls_assert_eq(a2[0], 1'b1);
        $vogls_assert_eq(a2[1], 1'b0);

        $vogls_assert_eq(a64[58], 1'b1);
        $vogls_assert_eq(a64[56], 1'b1);
        $vogls_assert_eq(a64[55], 1'b0);
    end
endmodule
