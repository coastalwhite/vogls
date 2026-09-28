`define X

module x();
    initial begin
		$vogls_assert_eq(0, 1
`ifdef X
		& 0
`endif
		);

		$vogls_assert_eq(1, 1
`ifdef Y
		& 0
`endif
		);
    end
endmodule
