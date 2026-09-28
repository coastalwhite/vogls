module x();
	function automatic [7:0] square;
		input [7:0] i;
		square = i * i;
	endfunction

	initial begin
		$vogls_assert_eq(0, square(0));
		$vogls_assert_eq(1, square(1));
		$vogls_assert_eq(4, square(2));
		$vogls_assert_eq(8'h39, square(8'hAB));
	end
endmodule
