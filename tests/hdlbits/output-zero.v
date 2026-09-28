module top_module(output zero);
    assign zero = 0;
endmodule

module tb();
	wire zero;
	top_module m(zero);
	initial #1 $vogls_assert_eq(zero, 0);
endmodule
