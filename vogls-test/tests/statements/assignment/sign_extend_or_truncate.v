module x();
	reg [3:0] a;
	initial begin
		a = 4'hA;
		$vogls_assert_eq(a, 4'hA);

		a = 7'h7B;
		$vogls_assert_eq(a, 4'hB);

		a = 2'h2;
		$vogls_assert_eq(a, 4'h2);
	end
endmodule
