module top_module( 
    input a, 
    input b, 
    output out
);
    assign out = ~(a ^ b);
endmodule

module tb();
	reg a, b;
	wire out;
	top_module m(
		a, b,
		out
	);
	initial begin
		a <= 0;
		b <= 0;
		#1
		$vogls_assert_eq(out, 1);

		a <= 1;
		#1
		$vogls_assert_eq(out, 0);

		b <= 1;
		#1
		$vogls_assert_eq(out, 1);

		a <= 0;
		#1
		$vogls_assert_eq(out, 0);
	end
endmodule
