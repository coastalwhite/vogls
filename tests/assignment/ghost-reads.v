module x();
	reg a = 0, b = 1, c = 0;

	initial begin
		$vogls_assert_eq(a[1], 1'bx);
		$vogls_assert_eq(b[1], 1'bx);
		$vogls_assert_eq(c[1], 1'bx);
		$vogls_assert_eq(a[-1], 1'bx);
		$vogls_assert_eq(b[-1], 1'bx);
		$vogls_assert_eq(c[-1], 1'bx);
		$vogls_assert_eq(a[$vogls_blackbox(1)], 1'bx);
		$vogls_assert_eq(b[$vogls_blackbox(1)], 1'bx);
		$vogls_assert_eq(c[$vogls_blackbox(1)], 1'bx);
		$vogls_assert_eq(a[$vogls_blackbox(-1)], 1'bx);
		$vogls_assert_eq(b[$vogls_blackbox(-1)], 1'bx);
		$vogls_assert_eq(c[$vogls_blackbox(-1)], 1'bx);
	end
endmodule
