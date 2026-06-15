module tb();
    integer a [double(1):0];

	function automatic integer double;
		input integer i;
		double = 2 * i;
	endfunction

    initial begin
        a[0] = 42;
        a[1] = 1337;
        a[2] = 0510;

        $vogls_assert_eq(a[0], 42);
        $vogls_assert_eq(a[1], 1337);
        $vogls_assert_eq(a[2], 0510);
    end
endmodule
