module tb();
	initial begin
		$vogls_assert_eq(~&(1'b0), 1'b1);
		$vogls_assert_eq(~&(1'b1), 1'b0);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(~&(1'bx), 1'bx);
		$vogls_assert_eq(~&(1'hz), 1'bx);
`endif

		$vogls_assert_eq(~&(7'h00), 1'b1);
		$vogls_assert_eq(~&(7'h7f), 1'b0);
		$vogls_assert_eq(~&(7'h76), 1'b1);
		$vogls_assert_eq(~&(7'h71), 1'b1);
		$vogls_assert_eq(~&(7'h43), 1'b1);
		$vogls_assert_eq(~&(7'h42), 1'b1);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(~&(7'b11x_zxx0), 1'b1);
		$vogls_assert_eq(~&(7'b11x_zxx1), 1'bx);
		$vogls_assert_eq(~&(7'b000_z0xz), 1'b1);
`endif

		$vogls_assert_eq(~&(31'h0000_0000), 1'b1);
		$vogls_assert_eq(~&(31'h7fff_ffff), 1'b0);
		$vogls_assert_eq(~&(31'h7c9f_138f), 1'b1);
		$vogls_assert_eq(~&(31'h5646_4580), 1'b1);
		$vogls_assert_eq(~&(31'h259e_0bdf), 1'b1);
		$vogls_assert_eq(~&(31'h7fda_2927), 1'b1);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(~&(31'b010_zzx1_zzxx_x101_1xxx_01z1_1zx1_x1z0), 1'b1);
		$vogls_assert_eq(~&(31'b111_zzx1_zzxx_x111_1xxx_11z1_1zx1_x1z1), 1'bx);
		$vogls_assert_eq(~&(31'bxx0_1xz0_z001_zxz1_1x0x_xz1x_110x_1x0z), 1'b1);
`endif

		$vogls_assert_eq(~&(32'h0000_0000), 1'b1);
		$vogls_assert_eq(~&(32'hffff_ffff), 1'b0);
		$vogls_assert_eq(~&(32'hc232_76ad), 1'b1);
		$vogls_assert_eq(~&(32'hd328_df00), 1'b1);
		$vogls_assert_eq(~&(32'hbb8b_c383), 1'b1);
		$vogls_assert_eq(~&(32'hfb9f_05fe), 1'b1);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(~&(32'b100x_1x01_11z1_x0z1_x1xz_11xz_z0z1_0000), 1'b1);
		$vogls_assert_eq(~&(32'b111x_1x11_11z1_x1z1_x1xz_11xz_z1z1_1111), 1'bx);
		$vogls_assert_eq(~&(32'bxz10_1z00_xz01_x001_0xz1_0xxz_0zzx_zxzz), 1'b1);
`endif

		$vogls_assert_eq(~&(33'h00_0000_0000), 1'b1);
		$vogls_assert_eq(~&(33'h01_ffff_ffff), 1'b0);
		$vogls_assert_eq(~&(33'h00_b41d_f51d), 1'b1);
		$vogls_assert_eq(~&(33'h00_5f57_a628), 1'b1);
		$vogls_assert_eq(~&(33'h00_1697_ceae), 1'b1);
		$vogls_assert_eq(~&(33'h00_7dac_b1f4), 1'b1);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(~&(33'bz_1xz1_1xx0_0zzz_1x1z_01xz_1z01_z1x1_zzxz), 1'b1);
		$vogls_assert_eq(~&(33'bz_1xz1_1xx1_1zzz_1x1z_11xz_1z11_z1x1_zzxz), 1'bx);
		$vogls_assert_eq(~&(33'b1_z1xz_1x00_1z1z_000z_zx0x_z10z_0zxz_xx1z), 1'b1);
`endif

		$vogls_assert_eq(~&(63'h0000_0000_0000_0000), 1'b1);
		$vogls_assert_eq(~&(63'h7fff_ffff_ffff_ffff), 1'b0);
		$vogls_assert_eq(~&(63'h0659_0089_04dc_e8e8), 1'b1);
		$vogls_assert_eq(~&(63'h5784_f71a_11d2_5e4a), 1'b1);
		$vogls_assert_eq(~&(63'h3dd9_8c51_e330_9621), 1'b1);
		$vogls_assert_eq(~&(63'h0eca_c749_8d2f_2e91), 1'b1);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(~&(63'b0x1_zzz1_xxz0_xxx1_101x_1x1z_z00z_x10z_x0x1_x11z_z1x0_x11z_0x0z_0xzx_0zz0_zz11), 1'b1);
		$vogls_assert_eq(~&(63'b1x1_zzz1_xxz1_xxx1_111x_1x1z_z11z_x11z_x1x1_x11z_z1x1_x11z_1x1z_1xzx_1zz1_zz11), 1'bx);
		$vogls_assert_eq(~&(63'b1x1_z1z1_01zz_110x_x0xx_0xzz_0x0x_10z0_010z_z000_zzz1_1011_zxzx_z110_z1zz_zzzz), 1'b1);
`endif

		$vogls_assert_eq(~&(64'h0000_0000_0000_0000), 1'b1);
		$vogls_assert_eq(~&(64'hffff_ffff_ffff_ffff), 1'b0);
		$vogls_assert_eq(~&(64'h0030_1be7_efd5_d7eb), 1'b1);
		$vogls_assert_eq(~&(64'hf036_d53b_4a62_3cff), 1'b1);
		$vogls_assert_eq(~&(64'he430_1867_253e_88f7), 1'b1);
		$vogls_assert_eq(~&(64'h6a1b_2e35_9eb2_f80a), 1'b1);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(~&(64'bx0zx_zzzx_001x_xxzx_xx11_zzz1_0zzz_00zx_zxz0_0z11_zz1x_1zxz_1z1z_z0zx_1x10_xx00), 1'b1);
		$vogls_assert_eq(~&(64'bx1zx_zzzx_111x_xxzx_xx11_zzz1_1zzz_11zx_zxz1_1z11_zz1x_1zxz_1z1z_z1zx_1x11_xx11), 1'bx);
		$vogls_assert_eq(~&(64'b00z0_xx10_0xx1_011z_0x0x_1xzx_xz1x_0010_01zx_10z0_z11x_101x_x110_1x1z_00zx_010x), 1'b1);
`endif

		$vogls_assert_eq(~&(65'h00_0000_0000_0000_0000), 1'b1);
		$vogls_assert_eq(~&(65'h01_ffff_ffff_ffff_ffff), 1'b0);
		$vogls_assert_eq(~&(65'h00_e5ea_0373_d0a9_9b7e), 1'b1);
		$vogls_assert_eq(~&(65'h01_de4b_cd69_747a_1997), 1'b1);
		$vogls_assert_eq(~&(65'h01_fd4b_59ad_56d1_9125), 1'b1);
		$vogls_assert_eq(~&(65'h00_c0ba_5a8c_905e_55ce), 1'b1);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(~&(65'bz_xx0x_zz10_zx1x_zx1z_z01x_1011_x11x_1z0x_10zz_1zzz_z10z_011x_0x1z_xxxx_x1xz_z1z0), 1'b1);
		$vogls_assert_eq(~&(65'bz_xx1x_zz11_zx1x_zx1z_z11x_1111_x11x_1z1x_11zz_1zzz_z11z_111x_1x1z_xxxx_x1xz_z1z1), 1'bx);
		$vogls_assert_eq(~&(65'bz_00z0_01z1_zzz1_0zz0_zz0z_1x10_0x11_z110_z0z0_0xz1_x00z_xx11_100x_z10z_1101_110z), 1'b1);
`endif

		$vogls_assert_eq(~&(127'h0000_0000_0000_0000_0000_0000_0000_0000), 1'b1);
		$vogls_assert_eq(~&(127'h7fff_ffff_ffff_ffff_ffff_ffff_ffff_ffff), 1'b0);
		$vogls_assert_eq(~&(127'h443a_d45c_0195_a501_be82_7758_b568_32d1), 1'b1);
		$vogls_assert_eq(~&(127'h7e2a_2363_419a_f329_81f5_e5e4_35bc_6c65), 1'b1);
		$vogls_assert_eq(~&(127'h06ea_4336_7117_2d42_5847_c38a_08a3_c8ca), 1'b1);
		$vogls_assert_eq(~&(127'h503d_8552_7c40_0e7d_5ef9_da61_421d_b2c6), 1'b1);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(~&(127'b100_0xxx_z1xz_z0zx_100z_xzzx_01x1_xzz1_0100_z01z_zxz0_zxz0_101z_zzz1_1z00_zxxz_1xxx_1z1z_zx01_xxx0_0110_11zx_zz00_x00z_10z1_xxx1_1xz0_x10x_0x1x_1010_01z1_z11x), 1'b1);
		$vogls_assert_eq(~&(127'b111_1xxx_z1xz_z1zx_111z_xzzx_11x1_xzz1_1111_z11z_zxz1_zxz1_111z_zzz1_1z11_zxxz_1xxx_1z1z_zx11_xxx1_1111_11zx_zz11_x11z_11z1_xxx1_1xz1_x11x_1x1x_1111_11z1_z11x), 1'bx);
		$vogls_assert_eq(~&(127'b01x_0zxz_zzxx_z11x_0000_z0z1_1111_1xx1_x0xx_zz11_x0z1_z1z1_0x11_x000_0xx1_z111_xx1x_z0z0_1x00_xxzz_zxxz_zzx0_1zxx_010x_zxzz_101z_xxz1_11xx_x00z_z100_zxx0_0x1x), 1'b1);
`endif

		$vogls_assert_eq(~&(128'h0000_0000_0000_0000_0000_0000_0000_0000), 1'b1);
		$vogls_assert_eq(~&(128'hffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff), 1'b0);
		$vogls_assert_eq(~&(128'h3230_cfa1_b449_a4fc_b83e_4659_0f61_68ed), 1'b1);
		$vogls_assert_eq(~&(128'he002_603b_75f6_b19a_71c9_4f30_1b26_c73c), 1'b1);
		$vogls_assert_eq(~&(128'h5b35_5508_171c_c81f_d4e3_b607_f4ff_f995), 1'b1);
		$vogls_assert_eq(~&(128'h5a33_6898_b87e_b1da_bc54_c193_66f6_e08c), 1'b1);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(~&(128'b011z_1x1z_0z1z_110z_0zxx_z0x1_z0x1_z100_1z0x_zz11_10z0_x00x_xxx0_x010_0z1z_xx0z_01x1_zz1z_1011_0z1z_1xz1_0xxx_z0zx_0x0z_xz1z_0111_01z0_0zx1_xxx0_1z10_zx0z_0000), 1'b1);
		$vogls_assert_eq(~&(128'b111z_1x1z_1z1z_111z_1zxx_z1x1_z1x1_z111_1z1x_zz11_11z1_x11x_xxx1_x111_1z1z_xx1z_11x1_zz1z_1111_1z1z_1xz1_1xxx_z1zx_1x1z_xz1z_1111_11z1_1zx1_xxx1_1z11_zx1z_1111), 1'bx);
		$vogls_assert_eq(~&(128'b1x10_z0zx_0x0z_011z_01x0_0x0z_x110_000z_0z10_xx11_0z1z_000x_1z1z_0zzx_110z_1000_0xz0_0x00_zx1z_z0zx_x00z_11z1_1zx0_xz11_1111_011z_z101_0010_1zzz_1zxx_11xz_zxx1), 1'b1);
`endif

		$vogls_assert_eq(~&(129'h00_0000_0000_0000_0000_0000_0000_0000_0000), 1'b1);
		$vogls_assert_eq(~&(129'h01_ffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff), 1'b0);
		$vogls_assert_eq(~&(129'h01_702e_77cd_21de_8d33_1202_ae25_a98a_d59a), 1'b1);
		$vogls_assert_eq(~&(129'h01_3c77_f502_5b4e_02bf_db3b_0acc_a73e_5724), 1'b1);
		$vogls_assert_eq(~&(129'h01_4e0f_a320_158e_7bc0_1db2_d21e_810c_07c6), 1'b1);
		$vogls_assert_eq(~&(129'h00_597b_4988_e125_c51e_e9f6_89d5_4a67_062e), 1'b1);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(~&(129'b0_x100_001x_0zxx_1x00_0001_xzx0_zzzx_z110_10z0_1x11_z11z_1011_1x1x_0001_x0z0_11zx_x010_xx10_z1zx_zx11_0xxz_1zzz_z0z0_x01z_0zx0_zx10_011z_xz1x_z10z_0z01_xxxz_0x1x), 1'b1);
		$vogls_assert_eq(~&(129'b1_x111_111x_1zxx_1x11_1111_xzx1_zzzx_z111_11z1_1x11_z11z_1111_1x1x_1111_x1z1_11zx_x111_xx11_z1zx_zx11_1xxz_1zzz_z1z1_x11z_1zx1_zx11_111z_xz1x_z11z_1z11_xxxz_1x1x), 1'bx);
		$vogls_assert_eq(~&(129'b0_1xxx_zzx1_1zzz_z1x1_1xz1_0x01_000z_x1xx_1zxz_z0z1_0xzx_zxzz_zxxz_011z_z0x1_0x0z_1110_00zz_x0zz_xxz0_x0z1_zxz0_x0zz_xzzx_1z1z_xxzx_z1xz_100z_0z0x_0100_0x0z_0xz1), 1'b1);
`endif

	end
endmodule
