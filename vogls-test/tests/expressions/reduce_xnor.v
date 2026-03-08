module tb();
	initial begin
		$vogls_assert_eq(~^(1'b0), 1'b1);
		$vogls_assert_eq(~^(1'b1), 1'b0);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(~^(1'bx), 1'bx);
		$vogls_assert_eq(~^(1'bz), 1'bx);
`endif

		$vogls_assert_eq(~^(7'h00), 1'b1);
		$vogls_assert_eq(~^(7'h7f), 1'b0);
		$vogls_assert_eq(~^(7'h0a), 1'b1);
		$vogls_assert_eq(~^(7'h5d), 1'b0);
		$vogls_assert_eq(~^(7'h2a), 1'b0);
		$vogls_assert_eq(~^(7'h76), 1'b0);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(~^(7'bz00_00x0), 1'bx);
		$vogls_assert_eq(~^(7'bz11_x111), 1'bx);
		$vogls_assert_eq(~^(7'bzzz_x1x0), 1'bx);
		$vogls_assert_eq(~^(7'b100_z111), 1'bx);
`endif

		$vogls_assert_eq(~^(31'h0000_0000), 1'b1);
		$vogls_assert_eq(~^(31'h7fff_ffff), 1'b0);
		$vogls_assert_eq(~^(31'h686a_f233), 1'b1);
		$vogls_assert_eq(~^(31'h2590_241c), 1'b1);
		$vogls_assert_eq(~^(31'h6f89_bc9a), 1'b1);
		$vogls_assert_eq(~^(31'h2dc4_08b0), 1'b0);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(~^(31'bz0z_xx0z_zz00_z000_zzx0_xx0x_zzx0_00zx), 1'bx);
		$vogls_assert_eq(~^(31'b111_z111_x111_x1zz_11z1_1z1z_z1z1_11z1), 1'bx);
		$vogls_assert_eq(~^(31'b11z_x1z0_xxz1_x0zz_0xz1_0zx1_xzzz_z1z1), 1'bx);
		$vogls_assert_eq(~^(31'bxz0_xxxx_z11x_00z0_0zx0_z1z1_x00z_x011), 1'bx);
`endif

		$vogls_assert_eq(~^(32'h0000_0000), 1'b1);
		$vogls_assert_eq(~^(32'hffff_ffff), 1'b1);
		$vogls_assert_eq(~^(32'hbc3e_e152), 1'b0);
		$vogls_assert_eq(~^(32'h4539_8707), 1'b1);
		$vogls_assert_eq(~^(32'h5d7d_a868), 1'b0);
		$vogls_assert_eq(~^(32'h5c3b_8931), 1'b0);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(~^(32'b00xx_zz0z_z0z0_0z0z_0000_zx00_xxzz_0z0x), 1'bx);
		$vogls_assert_eq(~^(32'b11zx_1111_1111_z111_xz11_1xx1_zz1z_x111), 1'bx);
		$vogls_assert_eq(~^(32'bx001_zxz1_1010_0zz0_01zx_1xz0_11xx_zx00), 1'bx);
		$vogls_assert_eq(~^(32'bx101_0z1z_zz0z_zxx0_zz01_0x01_0x0x_0x0z), 1'bx);
`endif

		$vogls_assert_eq(~^(33'h00_0000_0000), 1'b1);
		$vogls_assert_eq(~^(33'h01_ffff_ffff), 1'b0);
		$vogls_assert_eq(~^(33'h00_cb72_6588), 1'b0);
		$vogls_assert_eq(~^(33'h00_3074_1ade), 1'b0);
		$vogls_assert_eq(~^(33'h01_fbde_6deb), 1'b0);
		$vogls_assert_eq(~^(33'h01_a91f_deba), 1'b0);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(~^(33'bz_0zx0_0xz0_xx0x_xx0z_zx0z_0zzx_0000_zx00), 1'bx);
		$vogls_assert_eq(~^(33'bx_z1xz_zz11_zxzz_zzxx_11z1_zzxx_z11x_xxz1), 1'bx);
		$vogls_assert_eq(~^(33'b0_0111_zx0z_z1z0_001x_00z0_0x0z_1z1z_zx1z), 1'bx);
		$vogls_assert_eq(~^(33'b1_1x01_1z1z_0z00_x0zx_zzx1_00zz_xxz1_010z), 1'bx);
`endif

		$vogls_assert_eq(~^(63'h0000_0000_0000_0000), 1'b1);
		$vogls_assert_eq(~^(63'h7fff_ffff_ffff_ffff), 1'b0);
		$vogls_assert_eq(~^(63'h188c_eb23_b256_62b2), 1'b0);
		$vogls_assert_eq(~^(63'h6991_ac32_9921_aba0), 1'b0);
		$vogls_assert_eq(~^(63'h38f4_e85b_8770_15ec), 1'b1);
		$vogls_assert_eq(~^(63'h1f69_7ab4_8682_f9c6), 1'b0);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(~^(63'bzz0_0zxz_xz0z_zx0x_0xxx_xzzz_0z00_zz00_x0x0_0x00_0xxz_x00x_000x_xx00_00zz_00z0), 1'bx);
		$vogls_assert_eq(~^(63'bzx1_1z1x_11x1_1z1x_z11x_xxx1_x11z_z1zx_xx1z_1zxz_1x11_xxx1_11x1_zz11_1x1x_1x1z), 1'bx);
		$vogls_assert_eq(~^(63'bz01_xzx0_0x00_1000_11xz_xzxx_10z0_z11x_10zz_111x_xz01_0x1x_100z_z0x0_1000_111x), 1'bx);
		$vogls_assert_eq(~^(63'b100_1xx0_zzx1_xz0z_xx11_x100_11z1_10x0_1100_0x01_111x_zz1x_xzzz_xzxz_xz10_z01z), 1'bx);
`endif

		$vogls_assert_eq(~^(64'h0000_0000_0000_0000), 1'b1);
		$vogls_assert_eq(~^(64'hffff_ffff_ffff_ffff), 1'b1);
		$vogls_assert_eq(~^(64'h57de_0c26_8965_5354), 1'b1);
		$vogls_assert_eq(~^(64'ha444_c6b2_5e1e_cf38), 1'b0);
		$vogls_assert_eq(~^(64'h0efc_48ef_2dd1_01e5), 1'b1);
		$vogls_assert_eq(~^(64'h9666_bfa0_7b4c_8da8), 1'b0);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(~^(64'b0x0z_00z0_z0xz_z0x0_00xz_0x0x_x000_z00x_xzxx_x00z_000z_00xz_0000_0zz0_zz00_00xz), 1'bx);
		$vogls_assert_eq(~^(64'bx1z1_111z_z111_x1xz_zx1x_xx11_11z1_zzx1_11x1_zxx1_111z_1z11_111z_xzxx_1xzz_1zx1), 1'bx);
		$vogls_assert_eq(~^(64'b1z10_01z0_z101_1011_zx1x_0x1z_xz1x_x1x0_1xz1_z0x1_z0z0_x010_x00z_0xzx_0zxz_x101), 1'bx);
		$vogls_assert_eq(~^(64'bxxz1_zxzx_0z00_x00x_00xz_z1xz_x0z1_0xxx_xxx0_zzxx_x110_0011_0xx1_xzx0_xxz0_11xx), 1'bx);
`endif

		$vogls_assert_eq(~^(65'h00_0000_0000_0000_0000), 1'b1);
		$vogls_assert_eq(~^(65'h01_ffff_ffff_ffff_ffff), 1'b0);
		$vogls_assert_eq(~^(65'h01_8d90_14e1_1461_1660), 1'b0);
		$vogls_assert_eq(~^(65'h01_c710_4938_ae1a_ca3f), 1'b0);
		$vogls_assert_eq(~^(65'h00_f4b5_ddc4_dfc3_ab5b), 1'b1);
		$vogls_assert_eq(~^(65'h01_1f0d_f2d8_7951_51cf), 1'b0);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(~^(65'bz_x000_0x00_0000_0000_zx0x_x00z_0xzx_000z_x0zz_xxzx_x00z_x000_0zxz_z000_xx00_zz0z), 1'bx);
		$vogls_assert_eq(~^(65'bx_1xxx_x1z1_z11z_x1zx_1x11_1zxx_xx11_z111_1zxz_xx11_111z_z111_11xx_11xx_zzx1_1zx1), 1'bx);
		$vogls_assert_eq(~^(65'b1_01zz_z1zz_0xzx_xx1z_xxx0_11zx_z001_1zx0_zzzz_1xzz_11z1_zz0x_1011_zzzx_z0z1_xxxz), 1'bx);
		$vogls_assert_eq(~^(65'b1_0x01_1z0z_xx0x_zxx1_0z0x_zx11_x10z_101x_xxzz_x110_111x_1x00_0010_00z1_zxxx_1110), 1'bx);
`endif

		$vogls_assert_eq(~^(127'h0000_0000_0000_0000_0000_0000_0000_0000), 1'b1);
		$vogls_assert_eq(~^(127'h7fff_ffff_ffff_ffff_ffff_ffff_ffff_ffff), 1'b0);
		$vogls_assert_eq(~^(127'h429e_f2bb_8dfb_7dac_a1d1_1b84_5e92_7273), 1'b0);
		$vogls_assert_eq(~^(127'h3696_265e_4caf_75a2_d2cd_3384_c81d_9251), 1'b0);
		$vogls_assert_eq(~^(127'h74dc_0381_cfe5_f99d_9c79_968e_bf69_7ed2), 1'b0);
		$vogls_assert_eq(~^(127'h27c4_228c_f0d2_6d14_6b63_77a5_b6ee_914f), 1'b0);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(~^(127'bzz0_xxzx_x000_z0z0_zzx0_xxz0_z0xx_xxzx_zz00_z0xx_xxzz_xzx0_x0zx_zzz0_z000_z000_xz00_x0z0_xz00_z000_x0x0_zx0x_z0xx_0zzx_xzz0_zzxz_0000_0z0z_00zx_z000_x0xz_z000), 1'bx);
		$vogls_assert_eq(~^(127'bz11_zzxx_11z1_111x_1zz1_x1x1_x11z_z111_x111_111z_11z1_1z1x_111z_z111_111x_z1zz_z11z_1111_1x1z_1zz1_x1z1_z111_1x1x_z11z_x1xx_z1zz_11zz_1111_z1x1_x11x_xz1x_1z11), 1'bx);
		$vogls_assert_eq(~^(127'b000_0xz0_0xz1_0xxz_x0z1_xx10_1xx1_011x_0zz1_x11x_010z_0x00_z1xz_xzx0_zx1z_xzzz_1001_z110_z011_1001_zx10_xzz1_1xx1_10zx_zxz1_1xxx_10xz_z0z1_00xz_xxzz_x01x_001z), 1'bx);
		$vogls_assert_eq(~^(127'bx11_00z0_zz10_xx11_z1zx_0101_x111_0z0x_zzz0_111z_x0x1_110z_z001_00x0_zzxz_zz1x_1zzz_111z_1z1x_0010_x11x_11xz_x001_zxxx_10z0_z1xz_xxzx_1z00_xxz0_xx0x_0100_1xz0), 1'bx);
`endif

		$vogls_assert_eq(~^(128'h0000_0000_0000_0000_0000_0000_0000_0000), 1'b1);
		$vogls_assert_eq(~^(128'hffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff), 1'b1);
		$vogls_assert_eq(~^(128'h89b8_b898_209e_445d_b513_4acf_3372_dfac), 1'b0);
		$vogls_assert_eq(~^(128'h475d_3c0f_6a67_cb9f_0472_4fed_0616_921d), 1'b0);
		$vogls_assert_eq(~^(128'h9fd7_4e1b_1c8b_64f4_536e_9c87_3450_40fd), 1'b0);
		$vogls_assert_eq(~^(128'hfb0d_07b6_11c6_d3ec_8a17_3c64_574b_07f9), 1'b1);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(~^(128'bz0x0_xxz0_x00z_xz0x_x00x_0x00_00xx_0x00_z000_z000_0z0x_zz00_zxz0_000z_000x_0x0x_0000_xxz0_z00x_z0xx_000z_x0z0_0z00_0x00_0xxz_0000_z000_x0xz_z00x_0xzz_x0z0_xxz0), 1'bx);
		$vogls_assert_eq(~^(128'b1x1x_x111_1111_z1x1_x1x1_1zzx_x1x1_z1z1_111z_zx1z_zx1x_11x1_1111_xz1x_xz11_x1z1_1111_zzxz_x11x_x111_11z1_1xxx_z111_xzxz_xz1x_1zxz_111x_1zzz_xz1x_z1z1_x1xz_x11z), 1'bx);
		$vogls_assert_eq(~^(128'bzz11_zz00_x011_zzxz_11x1_0x00_0000_0xx1_xxxx_0010_0xz1_10zz_z110_z000_0zx1_xxxx_11x0_xzx1_1100_0x11_z0z1_zxxx_z11z_zzx1_11x0_1xz1_xz10_00z1_x110_z11x_x011_110x), 1'bx);
		$vogls_assert_eq(~^(128'bxzx1_0x01_zxzz_000z_xxxz_x010_x0xz_01z1_xxxz_0x10_0zx1_xzx0_z0z1_00zx_111z_010z_zxzx_zz0z_0z0z_11z1_zx1z_xzz0_z10z_11zx_0zxx_x011_xx10_1x11_xxzx_0000_z0x1_z010), 1'bx);
`endif

		$vogls_assert_eq(~^(129'h00_0000_0000_0000_0000_0000_0000_0000_0000), 1'b1);
		$vogls_assert_eq(~^(129'h01_ffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff), 1'b0);
		$vogls_assert_eq(~^(129'h01_8141_599a_0ffb_92f0_4cf8_7226_b19b_95e8), 1'b0);
		$vogls_assert_eq(~^(129'h00_d145_eda9_6efa_c68c_8642_3997_7e85_f33c), 1'b1);
		$vogls_assert_eq(~^(129'h01_6bd4_7a24_fe2e_4bdf_b6e2_6596_9579_31e5), 1'b0);
		$vogls_assert_eq(~^(129'h01_d08a_33ab_f9c7_ddd3_a95d_a46a_6230_8cd3), 1'b0);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(~^(129'bz_x0xz_00xx_zx0z_xz0z_0x0x_zxz0_0xx0_x00z_0xxz_z0xz_xzxz_0000_xzzz_zx00_00xz_xxx0_z0zx_z00x_00z0_0x0z_z0xz_zx0x_z0x0_x00z_0zx0_0x00_0zz0_0z00_x0zz_0xx0_00xx_xz00), 1'bx);
		$vogls_assert_eq(~^(129'bx_1111_x111_xx11_xxz1_1111_1zzx_xxz1_1x1x_11xx_1z1x_zzzz_zxxz_1z11_x1z1_zzxz_1x1x_11z1_z1z1_x1z1_1zzz_11zx_xx11_xxz1_x11z_xxx1_11z1_1z11_1zxz_1x11_11z1_1zzx_x111), 1'bx);
		$vogls_assert_eq(~^(129'b0_x0xz_0xx0_zx0x_10xz_11xx_10zx_xzxz_10z0_zxzx_x1xx_xx1x_z0xx_0xzx_xz0z_0zx1_x1zx_0zz1_xzzx_zxzx_xx10_zz01_11zx_0zzz_zzz0_z1xx_x0z0_x0xx_0x01_1101_0zxx_z1z1_zx00), 1'bx);
		$vogls_assert_eq(~^(129'b1_010z_1010_011z_zz0z_1101_zzxx_zx1z_x1x1_1x0z_x01z_0zx1_1xzz_xx10_0111_zzzz_zz11_zzxx_x10z_00z1_1000_x100_xz00_xz1x_z000_x101_1xxx_xz0z_zz11_x11z_1zx0_z0xx_xz0x), 1'bx);
`endif
	end
endmodule
