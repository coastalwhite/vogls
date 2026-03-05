module tb();
	reg a;
	reg[4:0] b;
	reg[32:0] c;
	initial begin
		a = 0; $vogls_assert_eq(~a, 1);
		a = 1; $vogls_assert_eq(~a, 0);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		a = 1'bx; $vogls_assert_eq(~a, 1'bx);
		a = 1'bz; $vogls_assert_eq(~a, 1'bx);
`endif

		b = 0; $vogls_assert_eq(~b, 5'b11111);
		b = 5'b11111; $vogls_assert_eq(~b, 0);
		b = 5'b01100; $vogls_assert_eq(~b, 5'b10011);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		b = 5'b010xz; $vogls_assert_eq(~b, 5'b101xx);
		b = 5'bz0x10; $vogls_assert_eq(~b, 5'bx1x01);
`endif

		c = 0; $vogls_assert_eq(~c, 33'h1_FFFF_FFFF);
		c = 33'h1_FFFF_FFFF; $vogls_assert_eq(~c, 0);
		c = 33'h1_BCD3_5213; $vogls_assert_eq(~c, 33'h0_432c_adec);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		c = 33'h0_0az2_3axf; $vogls_assert_eq(~c, 33'h1_f5xd_c5x0);
		c = 33'hx_5zzx_xxzb; $vogls_assert_eq(~c, 33'hx_axxx_xxx4);
`endif
	end
endmodule
