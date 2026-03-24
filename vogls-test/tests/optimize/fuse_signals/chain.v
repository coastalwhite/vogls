// vogls: verify-ir
module tb();
    reg a, b, c, d, e;
    assign a = b, b = c, c = d, d = e;

`ifndef __VOGLS_VERIFY_IR
    initial begin
        e = 1'b0;
        #0
		$vogls_assert_eq(a, 1'b0);
		$vogls_assert_eq(b, 1'b0);
		$vogls_assert_eq(c, 1'b0);
		$vogls_assert_eq(d, 1'b0);
		$vogls_assert_eq(e, 1'b0);
        e = 1'b1;
        #0
		$vogls_assert_eq(a, 1'b1);
		$vogls_assert_eq(b, 1'b1);
		$vogls_assert_eq(c, 1'b1);
		$vogls_assert_eq(d, 1'b1);
		$vogls_assert_eq(e, 1'b1);
    end
`endif
endmodule
