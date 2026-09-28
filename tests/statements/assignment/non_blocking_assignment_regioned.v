module x();
	reg a;
	wire b;

	initial begin
        a <= 'b0;
        #1
        a <= 'b1;
        b <= a;
        #1
        $vogls_assert_eq(b, 1'b0);
	end
endmodule
