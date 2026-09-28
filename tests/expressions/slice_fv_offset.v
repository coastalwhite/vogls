`define FV0 (32'bx & $vogls_blackbox(32'b0))

module tb;
	reg [15:0] mem [0:255];
	reg [255:0] wide;
	integer i;

	initial begin
		for (i = 0; i < 256; i = i + 1) mem[i] = i * 3 + 1;

		$vogls_assert_eq(mem[$vogls_blackbox(0)   + `FV0], 16'd1);
		$vogls_assert_eq(mem[$vogls_blackbox(7)   + `FV0], 16'd22);
		$vogls_assert_eq(mem[$vogls_blackbox(128) + `FV0], 16'd385);
		$vogls_assert_eq(mem[$vogls_blackbox(255) + `FV0], 16'd766);

		wide = {128'h0123456789abcdef_fedcba9876543210,
		        128'h00112233445566778899aabbccddeeff};

		$vogls_assert_eq(wide[128 * ($vogls_blackbox(0) + `FV0) +: 128],
		                 128'h00112233445566778899aabbccddeeff);
		$vogls_assert_eq(wide[128 * ($vogls_blackbox(1) + `FV0) +: 128],
		                 128'h0123456789abcdef_fedcba9876543210);

		$vogls_assert_eq(wide[($vogls_blackbox(60) + `FV0) +: 16], 16'h6778);
	end
endmodule
