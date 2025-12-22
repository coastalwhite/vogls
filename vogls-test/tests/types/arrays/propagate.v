module x();
    wire c [2:0];
	reg x, y, z;

    assign c[0] = x;
    assign c[1] = c[0] ^ y;
    assign c[2] = c[1] ^ z;

	initial begin
		x = 1'b0; y = 1'b0; z = 1'b0;
		#1 $vogls_assert_eq(c[2], 0);
		#1 x = 1'b1;
		#1 $vogls_assert_eq(c[2], 1);
		#1 y = 1'b1;
		#1 $vogls_assert_eq(c[2], 0);
		#1 z = 1'b1;
		#1 $vogls_assert_eq(c[2], 1);
	end
endmodule
