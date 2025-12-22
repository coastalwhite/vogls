module x();
	wire [ 71:0 ] LUT;
	assign LUT = { 8'h63, 64'b0 };
	initial #0 $vogls_assert_eq(8'h63, LUT[71-:8]);
endmodule
