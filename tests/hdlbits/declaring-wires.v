module top_module(
    input a,
    input b,
    input c,
    input d,
    output out,
    output out_n
); 
    wire w1, w2, w3;

    assign w1    = a & b,
           w2    = c & d,
           w3    = w1 | w2,
           out   = w3,
           out_n = ~w3;
endmodule

module tb();
	reg a, b, c, d;
	wire out, out_n;

	top_module m(
		a, b, c, d,
		out, out_n
	);
	initial begin
		a <= 0;
		b <= 0;
		c <= 0;
		d <= 0;
		#1
		$vogls_assert_eq(out, 0);
		$vogls_assert_eq(out_n, 1);

		a <= 1;
		b <= 1;
		c <= 0;
		d <= 0;
		#1
		$vogls_assert_eq(out, 1);
		$vogls_assert_eq(out_n, 0);

		a <= 0;
		b <= 1;
		c <= 1;
		d <= 0;
		#1
		$vogls_assert_eq(out, 0);
		$vogls_assert_eq(out_n, 1);
        
		a <= 0;
		b <= 0;
		c <= 1;
		d <= 1;
		#1
		$vogls_assert_eq(out, 1);
		$vogls_assert_eq(out_n, 0);
	end
endmodule