// vogls: mode=four-value-logic
module x();
	wire y;
	initial $vogls_assert_eq(y, 1'bz);
endmodule
