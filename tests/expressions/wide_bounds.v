module tb;
	reg [127:0] mem [0:3];
	reg [255:0] wide;
	wire [127:0] w0 = mem[$vogls_blackbox(1)];
	wire [127:0] w1 = mem[$vogls_blackbox(3)];
	wire [127:0] w2 = mem[$vogls_blackbox(4)];
	wire [127:0] w3 = wide[$vogls_blackbox(64) +: 128];
	wire [127:0] w4 = wide[$vogls_blackbox(192) +: 128];
`ifndef __VOGLS__TWO_VALUE_LOGIC
	wire [127:0] w5 = mem[$vogls_blackbox(1) + (32'bx | $vogls_blackbox(0))];
`endif

	initial begin
		mem[0] = 128'h00000000_00000000_00000000_00000000;
		mem[1] = 128'h11111111_22222222_33333333_44444444;
		mem[2] = 128'h55555555_66666666_77777777_88888888;
		mem[3] = 128'h99999999_aaaaaaaa_bbbbbbbb_cccccccc;
		wide = {128'hfedcba98_76543210_01234567_89abcdef, 128'h0f0f0f0f_f0f0f0f0_12121212_34343434};
		#1;
		$vogls_assert_eq(w0, 128'h11111111_22222222_33333333_44444444);
		$vogls_assert_eq(w1, 128'h99999999_aaaaaaaa_bbbbbbbb_cccccccc);
		$vogls_assert_eq(w3, 128'h01234567_89abcdef_0f0f0f0f_f0f0f0f0);
		$vogls_assert_eq(w4[63:0], 64'hfedcba98_76543210);
`ifdef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(w2, 128'h0);
		$vogls_assert_eq(w4[127:64], 64'h0);
`else
		$vogls_assert_eq(w2 === 128'bx, 1'b1);
		$vogls_assert_eq(w4[127:64] === 64'bx, 1'b1);
		$vogls_assert_eq(w5 === 128'bx, 1'b1);
`endif
	end
endmodule