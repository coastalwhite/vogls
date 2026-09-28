module top_module( 
    input a, 
    input b, 
    input c, 
    output out
);
    assign out = a & b & c;
endmodule

module tb();
	reg a, b, c;
	wire out;
	top_module m(
		a, b, c,
		out
	);
	initial begin
		a <= 0;
		b <= 0;
		c <= 0;
		#1
		$vogls_assert_eq(out, 0);

		a <= 1;
		#1
		$vogls_assert_eq(out, 0);

		b <= 1;
		#1
		$vogls_assert_eq(out, 0);

		c <= 1;
		#1
		$vogls_assert_eq(out, 1);

		a <= 0;
		#1
		$vogls_assert_eq(out, 0);
	end
endmodule
