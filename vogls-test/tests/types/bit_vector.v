module x();
	wire [31:0] w;
	assign w = 32'hDEAD_BEEF;
    initial $vogls_assert_eq(32'hDEAD_BEEF, w);
endmodule
