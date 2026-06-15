module top_module( 
    input a,b,c,
    output w,x,y,z
);
    assign
        w = a,
        x = b,
        y = b,
        z = c;
endmodule

module tb();
	reg a, b, c;
	wire w,x,y,z;
	top_module m(
		a, b, c,
		w, x, y, z
	);
	initial begin
		a <= 0;
		b <= 0;
		c <= 0;
		#1
		$vogls_assert_eq(w, 0);
		$vogls_assert_eq(x, 0);
		$vogls_assert_eq(y, 0);
		$vogls_assert_eq(z, 0);

		a <= 1;
		#1
		$vogls_assert_eq(w, 1);
		$vogls_assert_eq(x, 0);
		$vogls_assert_eq(y, 0);
		$vogls_assert_eq(z, 0);

		b <= 1;
		#1
		$vogls_assert_eq(w, 1);
		$vogls_assert_eq(x, 1);
		$vogls_assert_eq(y, 1);
		$vogls_assert_eq(z, 0);

		c <= 1;
		#1
		$vogls_assert_eq(w, 1);
		$vogls_assert_eq(x, 1);
		$vogls_assert_eq(y, 1);
		$vogls_assert_eq(z, 1);

		b <= 0;
		#1
		$vogls_assert_eq(w, 1);
		$vogls_assert_eq(x, 0);
		$vogls_assert_eq(y, 0);
		$vogls_assert_eq(z, 1);
		
		a <= 0;
		#1
		$vogls_assert_eq(w, 0);
		$vogls_assert_eq(x, 0);
		$vogls_assert_eq(y, 0);
		$vogls_assert_eq(z, 1);

		b <= 1;
		c <= 0;
		#1
		$vogls_assert_eq(w, 0);
		$vogls_assert_eq(x, 1);
		$vogls_assert_eq(y, 1);
		$vogls_assert_eq(z, 0);
	end
endmodule
