module x();
	reg [8:1] a;
	initial begin
		a = 1'b1;
		$vogls_assert_eq(a[1], 1);
		$vogls_assert_eq(a, 8'h01);
		a[1] = 1'b0;
		$vogls_assert_eq(a[1], 0);
		$vogls_assert_eq(a, 8'h00);
		a[7:2] = 6'b101010;
		$vogls_assert_eq(a, 8'b01010100);
	end
endmodule
