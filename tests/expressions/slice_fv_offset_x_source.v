// vogls: mode=four-value-logic
`define FV0 (32'bx & $vogls_blackbox(32'b0))

module tb;
	reg [255:0] w;

	initial begin
		w = 256'h0;
		w[5] = 1'bx;
		w[21] = 1'bx;

		$vogls_assert_eq(w[16 * ($vogls_blackbox(0) + `FV0) +: 16],
		                 16'b0000_0000_00x0_0000);
		$vogls_assert_eq(w[16 * ($vogls_blackbox(1) + `FV0) +: 16],
		                 16'b0000_0000_00x0_0000);

		w[47:32] = 16'hbeef;
		$vogls_assert_eq(w[16 * ($vogls_blackbox(2) + `FV0) +: 16], 16'hbeef);
	end
endmodule
