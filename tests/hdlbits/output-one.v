module top_module(output one);
    assign one = 1;
endmodule

module tb();
	wire one;
	top_module m(one);
    initial #1 $vogls_assert_eq(one, 1);
endmodule
