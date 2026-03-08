module tb();
	initial begin
		$vogls_assert_eq(~1'b0, 1);
		$vogls_assert_eq(~1'b1, 0);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(~1'bx, 1'bx);
		$vogls_assert_eq(~1'bz, 1'bx);
`endif

		$vogls_assert_eq(~5'b00000, 5'b11111);
		$vogls_assert_eq(~5'b11111, 5'b00000);
		$vogls_assert_eq(~5'b01100, 5'b10011);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(~5'b010xz, 5'b101xx);
		$vogls_assert_eq(~5'bz0x10, 5'bx1x01);
`endif

		$vogls_assert_eq(~33'h0, 33'h1_FFFF_FFFF);
		$vogls_assert_eq(~33'h1_FFFF_FFFF, 0);
		$vogls_assert_eq(~33'h1_BCD3_5213, 33'h0_432c_adec);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(~33'h0_0az2_3axf, 33'h1_f5xd_c5x0);
		$vogls_assert_eq(~33'hx_5zzx_xxzb, 33'hx_axxx_xxx4);
`endif

		$vogls_assert_eq(~(32'h0000_0000), 32'hffff_ffff);
		$vogls_assert_eq(~(32'hffff_ffff), 32'h0000_0000);
		$vogls_assert_eq(~(32'hc934_ae6d), 32'h36cb_5192);
		$vogls_assert_eq(~(32'h9877_9dda), 32'h6788_6225);
		$vogls_assert_eq(~(32'h54f8_c795), 32'hab07_386a);
		$vogls_assert_eq(~(32'h01f1_99f2), 32'hfe0e_660d);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(~(32'bz0xz_xx10_1zx0_z000_1100_xxzz_1xzx_z011), 32'bx1xx_xx01_0xx1_x111_0011_xxxx_0xxx_x100);
		$vogls_assert_eq(~(32'b011z_z101_1xzx_0100_0x1z_x0x1_0z00_001x), 32'b100x_x010_0xxx_1011_1x0x_x1x0_1x11_110x);
`endif

		$vogls_assert_eq(~(63'h0000_0000_0000_0000), 63'h7fff_ffff_ffff_ffff);
		$vogls_assert_eq(~(63'h7fff_ffff_ffff_ffff), 63'h0000_0000_0000_0000);
		$vogls_assert_eq(~(63'h56de_2af8_35e1_b622), 63'h2921_d507_ca1e_49dd);
		$vogls_assert_eq(~(63'h4464_2839_f0a1_373f), 63'h3b9b_d7c6_0f5e_c8c0);
		$vogls_assert_eq(~(63'h50fe_2280_e2c4_0554), 63'h2f01_dd7f_1d3b_faab);
		$vogls_assert_eq(~(63'h500e_8b99_c21d_f8b7), 63'h2ff1_7466_3de2_0748);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(~(63'bz11_zz11_0x01_z111_zx0x_10xx_zx01_1010_zzzx_z011_0z01_zzz0_00z0_x0z1_1111_z0zx), 63'bx00_xx00_1x10_x000_xx1x_01xx_xx10_0101_xxxx_x100_1x10_xxx1_11x1_x1x0_0000_x1xx);
		$vogls_assert_eq(~(63'bx1z_0zzx_110z_00zx_0xzz_x0z0_z100_100x_0010_xzzz_z00x_zxx0_1z1z_zxz0_x110_1xx0), 63'bx0x_1xxx_001x_11xx_1xxx_x1x1_x011_011x_1101_xxxx_x11x_xxx1_0x0x_xxx1_x001_0xx1);
`endif

		$vogls_assert_eq(~(64'h0000_0000_0000_0000), 64'hffff_ffff_ffff_ffff);
		$vogls_assert_eq(~(64'hffff_ffff_ffff_ffff), 64'h0000_0000_0000_0000);
		$vogls_assert_eq(~(64'hb3df_cf0e_4370_a62a), 64'h4c20_30f1_bc8f_59d5);
		$vogls_assert_eq(~(64'h0de4_4caa_1e23_37e3), 64'hf21b_b355_e1dc_c81c);
		$vogls_assert_eq(~(64'h471c_c9e4_8543_0946), 64'hb8e3_361b_7abc_f6b9);
		$vogls_assert_eq(~(64'h24e9_297a_6116_7c3e), 64'hdb16_d685_9ee9_83c1);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(~(64'bx1xz_111z_z1x1_x0x0_010z_z1zz_1z00_0z11_zx0z_x0x0_1000_0xx0_xzxz_1z0z_z011_x10z), 64'bx0xx_000x_x0x0_x1x1_101x_x0xx_0x11_1x00_xx1x_x1x1_0111_1xx1_xxxx_0x1x_x100_x01x);
		$vogls_assert_eq(~(64'b1xxx_100x_xzz1_z00x_z101_1xx0_z1z1_1z01_111x_00z1_0x1x_x1xx_0zzx_011z_z1z0_0xx0), 64'b0xxx_011x_xxx0_x11x_x010_0xx1_x0x0_0x10_000x_11x0_1x0x_x0xx_1xxx_100x_x0x1_1xx1);
`endif

		$vogls_assert_eq(~(65'h00_0000_0000_0000_0000), 65'h01_ffff_ffff_ffff_ffff);
		$vogls_assert_eq(~(65'h01_ffff_ffff_ffff_ffff), 65'h00_0000_0000_0000_0000);
		$vogls_assert_eq(~(65'h00_2b03_9134_7a22_ebd8), 65'h01_d4fc_6ecb_85dd_1427);
		$vogls_assert_eq(~(65'h01_8dc5_1a77_c06b_8af5), 65'h00_723a_e588_3f94_750a);
		$vogls_assert_eq(~(65'h00_9df7_f4f4_63bd_1282), 65'h01_6208_0b0b_9c42_ed7d);
		$vogls_assert_eq(~(65'h01_1d33_7015_27ad_83f6), 65'h00_e2cc_8fea_d852_7c09);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(~(65'bz_1z00_0zz0_1z0x_zx01_zzx0_00z0_1x01_00x0_x10x_zx0x_xz0z_xxzz_110z_01z0_zzx1_1000), 65'bx_0x11_1xx1_0x1x_xx10_xxx1_11x1_0x10_11x1_x01x_xx1x_xx1x_xxxx_001x_10x1_xxx0_0111);
		$vogls_assert_eq(~(65'b1_1z1z_z0x0_z000_0z11_z1z1_00xx_111x_0010_zz11_1x11_0zx0_0111_xx1z_1100_0110_0xx1), 65'b0_0x0x_x1x1_x111_1x00_x0x0_11xx_000x_1101_xx00_0x00_1xx1_1000_xx0x_0011_1001_1xx0);
`endif

		$vogls_assert_eq(~(127'h0000_0000_0000_0000_0000_0000_0000_0000), 127'h7fff_ffff_ffff_ffff_ffff_ffff_ffff_ffff);
		$vogls_assert_eq(~(127'h7fff_ffff_ffff_ffff_ffff_ffff_ffff_ffff), 127'h0000_0000_0000_0000_0000_0000_0000_0000);
		$vogls_assert_eq(~(127'h59d6_0746_0233_fc13_1cf3_12bb_9b92_306f), 127'h2629_f8b9_fdcc_03ec_e30c_ed44_646d_cf90);
		$vogls_assert_eq(~(127'h113f_5a9d_d04a_00a6_f9b6_768f_5190_b0dc), 127'h6ec0_a562_2fb5_ff59_0649_8970_ae6f_4f23);
		$vogls_assert_eq(~(127'h68fa_3a16_8c2c_2201_7ba0_b86e_3533_885c), 127'h1705_c5e9_73d3_ddfe_845f_4791_cacc_77a3);
		$vogls_assert_eq(~(127'h04db_2428_7c72_f16a_937b_cba6_21dc_5e05), 127'h7b24_dbd7_838d_0e95_6c84_3459_de23_a1fa);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(~(127'bx0z_00z1_1001_zxzx_0zz0_00xz_01xx_01xx_x0xz_xx10_1x0x_xz10_x111_xx11_1001_1x11_01z1_0000_10x0_1z11_101z_11xz_1zxx_0x1z_110x_0x0z_1101_0x11_zzx1_0zx0_x0xz_zz11), 127'bx1x_11x0_0110_xxxx_1xx1_11xx_10xx_10xx_x1xx_xx01_0x1x_xx01_x000_xx00_0110_0x00_10x0_1111_01x1_0x00_010x_00xx_0xxx_1x0x_001x_1x1x_0010_1x00_xxx0_1xx1_x1xx_xx00);
		$vogls_assert_eq(~(127'bx01_xxxz_z11x_x1zz_1z0x_0zxz_11zz_x0xz_1xxz_xz10_x0xx_01x1_z10x_1z01_xx1z_1zz1_1110_1z1z_zxz0_z0x1_xz1z_1000_0x10_x000_0011_11z1_10z1_1xxx_zz1z_0z1x_1xz0_zz10), 127'bx10_xxxx_x00x_x0xx_0x1x_1xxx_00xx_x1xx_0xxx_xx01_x1xx_10x0_x01x_0x10_xx0x_0xx0_0001_0x0x_xxx1_x1x0_xx0x_0111_1x01_x111_1100_00x0_01x0_0xxx_xx0x_1x0x_0xx1_xx01);
`endif

		$vogls_assert_eq(~(128'h0000_0000_0000_0000_0000_0000_0000_0000), 128'hffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff);
		$vogls_assert_eq(~(128'hffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff), 128'h0000_0000_0000_0000_0000_0000_0000_0000);
		$vogls_assert_eq(~(128'h7754_1013_5301_ce22_373c_a7de_6812_21b6), 128'h88ab_efec_acfe_31dd_c8c3_5821_97ed_de49);
		$vogls_assert_eq(~(128'h0575_a996_8563_df83_9f17_1a24_841f_08e0), 128'hfa8a_5669_7a9c_207c_60e8_e5db_7be0_f71f);
		$vogls_assert_eq(~(128'h5c02_dda3_3bf6_09af_80ec_2ec0_64c7_06d5), 128'ha3fd_225c_c409_f650_7f13_d13f_9b38_f92a);
		$vogls_assert_eq(~(128'hf87f_d406_5f55_f09e_ef96_c0f1_c804_8728), 128'h0780_2bf9_a0aa_0f61_1069_3f0e_37fb_78d7);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(~(128'b1x1z_1x11_x0z1_z11z_xxz0_xx0z_00x1_0z0x_zxzx_x0x0_0x11_1x00_x1x1_z1z0_z111_1zz1_x10x_1z10_0x0x_z1x0_xx1z_zx0z_zx11_11x1_xx1z_001z_x1z0_zx11_zx0x_x0xz_xxzx_0zxx), 128'b0x0x_0x00_x1x0_x00x_xxx1_xx1x_11x0_1x1x_xxxx_x1x1_1x00_0x11_x0x0_x0x1_x000_0xx0_x01x_0x01_1x1x_x0x1_xx0x_xx1x_xx00_00x0_xx0x_110x_x0x1_xx00_xx1x_x1xx_xxxx_1xxx);
		$vogls_assert_eq(~(128'bxx0x_zx0z_1z1x_1xxz_1zz0_1010_1010_z11z_xz1z_1z10_zx1z_001x_0100_zxz0_x0x1_x0zz_xxzz_x1x0_z001_x01z_z001_110x_x0x0_1zzz_zz11_zx11_11xz_0x11_zzz0_zx00_z111_zz1z), 128'bxx1x_xx1x_0x0x_0xxx_0xx1_0101_0101_x00x_xx0x_0x01_xx0x_110x_1011_xxx1_x1x0_x1xx_xxxx_x0x1_x110_x10x_x110_001x_x1x1_0xxx_xx00_xx00_00xx_1x00_xxx1_xx11_x000_xx0x);
`endif

		$vogls_assert_eq(~(129'h00_0000_0000_0000_0000_0000_0000_0000_0000), 129'h01_ffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff);
		$vogls_assert_eq(~(129'h01_ffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff), 129'h00_0000_0000_0000_0000_0000_0000_0000_0000);
		$vogls_assert_eq(~(129'h00_db95_ca86_a83b_1423_7890_b1f2_4474_6cbf), 129'h01_246a_3579_57c4_ebdc_876f_4e0d_bb8b_9340);
		$vogls_assert_eq(~(129'h01_b3b7_de91_7fd9_7970_d582_0540_45df_e5d1), 129'h00_4c48_216e_8026_868f_2a7d_fabf_ba20_1a2e);
		$vogls_assert_eq(~(129'h01_da72_29c0_4513_ba9b_18aa_7cd6_316f_5ae6), 129'h00_258d_d63f_baec_4564_e755_8329_ce90_a519);
		$vogls_assert_eq(~(129'h00_6fc2_133a_acd5_2902_f792_feef_1bb0_b49a), 129'h01_903d_ecc5_532a_d6fd_086d_0110_e44f_4b65);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(~(129'bz_zxx0_x00z_1xxx_x1z1_xz10_zx01_001x_1zx0_11zz_z10x_z11z_01zx_x011_x101_zx0x_0x1x_z1zz_1zzz_z1zz_10zx_1x1z_1zz1_z1x1_1zzx_0z1x_x111_z0z0_xx10_xx0x_1xz0_xz0x_xxxz), 129'bx_xxx1_x11x_0xxx_x0x0_xx01_xx10_110x_0xx1_00xx_x01x_x00x_10xx_x100_x010_xx1x_1x0x_x0xx_0xxx_x0xx_01xx_0x0x_0xx0_x0x0_0xxx_1x0x_x000_x1x1_xx01_xx1x_0xx1_xx1x_xxxx);
		$vogls_assert_eq(~(129'b0_zz0x_zx0x_1z10_1z00_zzzx_1xzz_x00z_x10z_1110_xz01_zz1x_1xzx_xz10_x11z_z111_0zz0_0x10_11x1_1z1z_z01x_011z_x1zx_z110_zx10_0xzz_xzz1_0zx1_0xz0_0zx1_001x_zxxz_xx0x), 129'b1_xx1x_xx1x_0x01_0x11_xxxx_0xxx_x11x_x01x_0001_xx10_xx0x_0xxx_xx01_x00x_x000_1xx1_1x01_00x0_0x0x_x10x_100x_x0xx_x001_xx01_1xxx_xxx0_1xx0_1xx1_1xx0_110x_xxxx_xx1x);
`endif
	end
endmodule
