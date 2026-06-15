module tb();
	initial begin
		$vogls_assert_eq(~|(1'b0), 1'b1);
		$vogls_assert_eq(~|(1'b1), 1'b0);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(~|(1'bx), 1'bx);
		$vogls_assert_eq(~|(1'bz), 1'bx);
`endif

		$vogls_assert_eq(~|(7'h00), 1'b1);
		$vogls_assert_eq(~|(7'h7f), 1'b0);
		$vogls_assert_eq(~|(7'h28), 1'b0);
		$vogls_assert_eq(~|(7'h7a), 1'b0);
		$vogls_assert_eq(~|(7'h23), 1'b0);
		$vogls_assert_eq(~|(7'h7f), 1'b0);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(~|(7'b0zz_00x0), 1'bx);
		$vogls_assert_eq(~|(7'bx1x_00zx), 1'b0);
`endif

		$vogls_assert_eq(~|(31'h0000_0000), 1'b1);
		$vogls_assert_eq(~|(31'h7fff_ffff), 1'b0);
		$vogls_assert_eq(~|(31'h276a_eb4e), 1'b0);
		$vogls_assert_eq(~|(31'h2f8f_41be), 1'b0);
		$vogls_assert_eq(~|(31'h547d_1a4c), 1'b0);
		$vogls_assert_eq(~|(31'h6531_6881), 1'b0);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(~|(31'bxxx_0z00_0xz0_xz00_0xxz_x00z_zz0x_0z0x), 1'bx);
		$vogls_assert_eq(~|(31'b1xx_00xz_0xzx_xxz0_zxzx_xzzz_1zxx_01z0), 1'b0);
`endif

		$vogls_assert_eq(~|(32'h0000_0000), 1'b1);
		$vogls_assert_eq(~|(32'hffff_ffff), 1'b0);
		$vogls_assert_eq(~|(32'h8225_b4b6), 1'b0);
		$vogls_assert_eq(~|(32'h3f2c_924f), 1'b0);
		$vogls_assert_eq(~|(32'h6864_d593), 1'b0);
		$vogls_assert_eq(~|(32'h8be0_73b7), 1'b0);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(~|(32'bz0z0_xxzx_x00x_00z0_z000_z0zx_xz00_z0x0), 1'bx);
		$vogls_assert_eq(~|(32'bxx00_10x1_xzx1_0z10_01zz_0zzx_0x0x_1zx1), 1'b0);
`endif

		$vogls_assert_eq(~|(33'h00_0000_0000), 1'b1);
		$vogls_assert_eq(~|(33'h01_ffff_ffff), 1'b0);
		$vogls_assert_eq(~|(33'h01_ed04_6d14), 1'b0);
		$vogls_assert_eq(~|(33'h01_b412_510e), 1'b0);
		$vogls_assert_eq(~|(33'h00_0e2d_c898), 1'b0);
		$vogls_assert_eq(~|(33'h00_9747_8e12), 1'b0);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(~|(33'b0_xzxx_z00x_xzz0_00z0_xxz0_xz00_z00x_zzxx), 1'bx);
		$vogls_assert_eq(~|(33'b0_1xx1_1xz1_zx11_z0xx_0zzz_1x11_1z0x_xx1z), 1'b0);
`endif

		$vogls_assert_eq(~|(63'h0000_0000_0000_0000), 1'b1);
		$vogls_assert_eq(~|(63'h7fff_ffff_ffff_ffff), 1'b0);
		$vogls_assert_eq(~|(63'h4d68_7b0e_c6ad_fe1c), 1'b0);
		$vogls_assert_eq(~|(63'h1fd6_9aad_dd9e_0257), 1'b0);
		$vogls_assert_eq(~|(63'h38de_2b56_1157_f10f), 1'b0);
		$vogls_assert_eq(~|(63'h3f7f_5993_b1aa_250e), 1'b0);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(~|(63'b00z_zz00_00zx_x0zx_zz00_zx0x_0z0x_00z0_z000_0000_zzz0_zx0x_0xx0_zzx0_z0xx_000x), 1'bx);
		$vogls_assert_eq(~|(63'b1xx_0xxz_z00x_01xx_100x_0110_0x0z_0110_0z11_111z_1zz0_z101_z00x_z1z0_11x1_11z1), 1'b0);
`endif

		$vogls_assert_eq(~|(64'h0000_0000_0000_0000), 1'b1);
		$vogls_assert_eq(~|(64'hffff_ffff_ffff_ffff), 1'b0);
		$vogls_assert_eq(~|(64'h8aa6_80fd_54db_749a), 1'b0);
		$vogls_assert_eq(~|(64'h2ff2_caad_0a93_e945), 1'b0);
		$vogls_assert_eq(~|(64'h1bfe_6baf_0141_e9de), 1'b0);
		$vogls_assert_eq(~|(64'h3c16_e5af_cbfb_6ec0), 1'b0);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(~|(64'b000x_xzzx_0xz0_0zxx_00x0_x00x_z0x0_0xx0_00zx_x0zz_0zxz_0xx0_00zx_00xx_0xz0_0000), 1'bx);
		$vogls_assert_eq(~|(64'b0x00_010z_110z_z001_x001_xz0x_00x1_x1x1_zzxx_0x1x_zx01_zxx0_1011_xzxx_1zz1_z001), 1'b0);
`endif

		$vogls_assert_eq(~|(65'h00_0000_0000_0000_0000), 1'b1);
		$vogls_assert_eq(~|(65'h01_ffff_ffff_ffff_ffff), 1'b0);
		$vogls_assert_eq(~|(65'h00_ffc7_8278_91d7_7254), 1'b0);
		$vogls_assert_eq(~|(65'h01_6bf4_ffbf_3229_adf4), 1'b0);
		$vogls_assert_eq(~|(65'h01_aa65_19aa_68c3_6afc), 1'b0);
		$vogls_assert_eq(~|(65'h01_3573_34b7_6f51_43b2), 1'b0);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(~|(65'bx_0000_0x00_x00x_zzx0_z0zz_xxz0_x00x_z00z_0zz0_0zxx_0x0x_0000_x0x0_0x00_xx0x_x0zx), 1'bx);
		$vogls_assert_eq(~|(65'b1_0xx1_x00z_0z10_0x11_1xxx_z10z_1xxz_zzx0_z0xz_1101_000x_1x1x_01x0_x1xx_01x1_00xx), 1'b0);
`endif

		$vogls_assert_eq(~|(127'h0000_0000_0000_0000_0000_0000_0000_0000), 1'b1);
		$vogls_assert_eq(~|(127'h7fff_ffff_ffff_ffff_ffff_ffff_ffff_ffff), 1'b0);
		$vogls_assert_eq(~|(127'h533a_f8ce_0059_e980_dbcf_40fd_3fa5_c0ff), 1'b0);
		$vogls_assert_eq(~|(127'h0c8a_2073_7fd3_0ace_4076_6900_a43c_0da1), 1'b0);
		$vogls_assert_eq(~|(127'h26c9_2e44_711a_ff54_f396_f46e_c81f_37fd), 1'b0);
		$vogls_assert_eq(~|(127'h1fb6_5871_b240_5b95_4573_9387_5107_433c), 1'b0);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(~|(127'b000_00x0_zxxz_0x00_000z_zz00_00x0_0z0x_0xx0_zzzz_0zzx_000z_z00x_00zz_z0zx_z00x_0xz0_00x0_00zz_z0x0_000z_x000_0z0z_0z00_z00z_x0x0_zx00_00x0_zx00_z0z0_0z0x_x0zx), 1'bx);
		$vogls_assert_eq(~|(127'bxxx_zxz0_111x_1xzz_1100_zx1x_10z1_x101_zz00_100z_1010_011x_1xx0_zzx1_10zz_zx1x_1z11_0zx0_xz01_1zzx_z010_x0z1_1111_zxz1_100x_111x_01x1_zxz0_1z00_1111_xxz0_00xz), 1'b0);
`endif

		$vogls_assert_eq(~|(128'h0000_0000_0000_0000_0000_0000_0000_0000), 1'b1);
		$vogls_assert_eq(~|(128'hffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff), 1'b0);
		$vogls_assert_eq(~|(128'h6fa2_e0bd_c94d_27d8_6e81_1cfa_f0c8_24b1), 1'b0);
		$vogls_assert_eq(~|(128'h082a_2c0c_f199_98e7_bf78_9bcf_69fa_c938), 1'b0);
		$vogls_assert_eq(~|(128'h1778_88d4_7038_e8ba_8795_3f60_6da2_7a0c), 1'b0);
		$vogls_assert_eq(~|(128'he580_1d77_d76b_7bea_2ca4_0a8d_5769_231d), 1'b0);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(~|(128'b00xx_xx00_000z_xzz0_zx0z_0000_xxxx_x000_0xzz_00z0_xxz0_xz0z_z0zx_zz0z_0x0z_xxz0_0zzx_zxzx_zx00_x00z_0z0x_0z0z_z000_xx0x_00x0_xxxz_0z00_000x_0z0z_z00x_0000_00zz), 1'bx);
		$vogls_assert_eq(~|(128'b01xx_1x00_0z1x_z0zz_x0zz_x0zx_zz0z_xxxx_z11x_z0z1_zxxx_0011_0z0x_1zx1_10xz_001x_1x0z_z1zx_0x00_zxx1_zz00_x1xz_z1zz_xx1z_1z1z_x01x_1xx1_10xx_01zx_zz00_zx1z_z1zx), 1'b0);
`endif

		$vogls_assert_eq(~|(129'h00_0000_0000_0000_0000_0000_0000_0000_0000), 1'b1);
		$vogls_assert_eq(~|(129'h01_ffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff), 1'b0);
		$vogls_assert_eq(~|(129'h00_5f8f_f0d3_cd8e_09f6_9d7d_6644_ac70_70f3), 1'b0);
		$vogls_assert_eq(~|(129'h01_931b_1d34_5400_df5b_6569_49f2_c1eb_42c3), 1'b0);
		$vogls_assert_eq(~|(129'h01_cf99_5217_cc5f_5ad4_80f9_d4bf_3740_49b5), 1'b0);
		$vogls_assert_eq(~|(129'h00_21c6_3bfb_ab96_13e4_3421_087d_6029_d9e9), 1'b0);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(~|(129'b0_0xxz_z0zx_0xx0_00x0_xx00_00x0_z000_0xzz_0z00_00zx_xxx0_0x00_0z0x_xxzz_00z0_00z0_zzz0_xz00_00xz_xzzx_0000_000z_z000_00z0_00xx_000z_zxz0_0z00_zx0z_000z_xz0z_0x00), 1'bx);
		$vogls_assert_eq(~|(129'bz_zz1x_0xzz_x1xx_0xzz_z0zx_10z1_xzx1_xzxz_x1z1_z0xx_1000_x0xz_0x1z_11z0_xz10_0zzx_z0z1_xz01_zz01_100z_0z00_xzxz_0x0z_x1z1_zzzx_110z_0z0z_1xz1_0001_zz1z_x001_1xxz), 1'b0);
`endif
	end
endmodule
