module x();
    initial begin
        $vogls_assert_eq(1'b0 * 1'b0, 1'b0);
        $vogls_assert_eq(1'b1 * 1'b0, 1'b0);
        $vogls_assert_eq(1'b0 * 1'b1, 1'b0);
        $vogls_assert_eq(1'b1 * 1'b1, 1'b1);
`ifndef __VOGLS__TWO_VALUE_LOGIC
        $vogls_assert_eq(1'bx * 1'b0, 1'bx);
        $vogls_assert_eq(1'bz * 1'b0, 1'bx);
        $vogls_assert_eq(1'bx * 1'b1, 1'bx);
        $vogls_assert_eq(1'bz * 1'b1, 1'bx);
        $vogls_assert_eq(1'b0 * 1'bx, 1'bx);
        $vogls_assert_eq(1'b0 * 1'bz, 1'bx);
        $vogls_assert_eq(1'b1 * 1'bx, 1'bx);
        $vogls_assert_eq(1'b1 * 1'bz, 1'bx);
`endif

		$vogls_assert_eq(7'h00 * 7'h00, 7'h00);
		$vogls_assert_eq(7'h00 * 7'h7f, 7'h00);
		$vogls_assert_eq(7'h7f * 7'h00, 7'h00);
		$vogls_assert_eq(7'h7f * 7'h7f, 7'h01);
		$vogls_assert_eq(7'h1a * 7'h17, 7'h56);
		$vogls_assert_eq(7'h61 * 7'h18, 7'h18);
		$vogls_assert_eq(7'h5b * 7'h58, 7'h48);
		$vogls_assert_eq(7'h43 * 7'h0b, 7'h61);
		$vogls_assert_eq(7'h75 * 7'h1f, 7'h2b);
		$vogls_assert_eq(7'h60 * 7'h14, 7'h00);
		$vogls_assert_eq(7'h4b * 7'h5c, 7'h74);
		$vogls_assert_eq(7'h31 * 7'h11, 7'h41);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(7'b01x_010z * 7'b100_0111, 7'bxxx_xxxx);
		$vogls_assert_eq(7'b001_0010 * 7'bzx1_xx1x, 7'bxxx_xxxx);
		$vogls_assert_eq(7'b111_zzx1 * 7'bx01_0xzx, 7'bxxx_xxxx);
		$vogls_assert_eq(7'b01x_1zzz * 7'b1x1_1xzz, 7'bxxx_xxxx);
`endif

		$vogls_assert_eq(31'h0000_0000 * 31'h0000_0000, 31'h0000_0000);
		$vogls_assert_eq(31'h0000_0000 * 31'h7fff_ffff, 31'h0000_0000);
		$vogls_assert_eq(31'h7fff_ffff * 31'h0000_0000, 31'h0000_0000);
		$vogls_assert_eq(31'h7fff_ffff * 31'h7fff_ffff, 31'h0000_0001);
		$vogls_assert_eq(31'h5cab_cc97 * 31'h3825_67b8, 31'h549e_cd88);
		$vogls_assert_eq(31'h2369_b584 * 31'h7e57_0ddf, 31'h5f28_d1fc);
		$vogls_assert_eq(31'h1745_d6d8 * 31'h0c0f_d195, 31'h46b4_63b8);
		$vogls_assert_eq(31'h1c11_f735 * 31'h2720_9bdf, 31'h4df3_6e2b);
		$vogls_assert_eq(31'h28f4_9481 * 31'h6c12_ace8, 31'h4e7f_40e8);
		$vogls_assert_eq(31'h1043_5a10 * 31'h6280_1c45, 31'h6801_0650);
		$vogls_assert_eq(31'h61b1_cd22 * 31'h77d2_1e02, 31'h2d51_9644);
		$vogls_assert_eq(31'h405c_acec * 31'h02f0_6b90, 31'h65a7_e8c0);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(31'b0xx_0xz1_z0x1_0x11_x10x_z00x_x101_00z0 * 31'b010_0000_0011_0001_1101_0111_0101_0000, 31'bxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx);
		$vogls_assert_eq(31'b011_0110_1101_1000_0011_1001_0011_1010 * 31'b1z1_xz11_xzxz_z011_0x01_1000_100x_01xz, 31'bxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx);
		$vogls_assert_eq(31'b1z1_zz10_0zxz_zz00_0zx0_111z_1z1x_z10z * 31'b000_011z_zz1z_01z0_zxzx_zz11_x100_x00z, 31'bxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx);
		$vogls_assert_eq(31'b100_1001_z010_0zxx_1x1x_z1xz_x00z_001x * 31'b1x0_1xx1_zx0x_01x0_01xx_1x1x_zx00_zx00, 31'bxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx);
`endif

		$vogls_assert_eq(32'h0000_0000 * 32'h0000_0000, 32'h0000_0000);
		$vogls_assert_eq(32'h0000_0000 * 32'hffff_ffff, 32'h0000_0000);
		$vogls_assert_eq(32'hffff_ffff * 32'h0000_0000, 32'h0000_0000);
		$vogls_assert_eq(32'hffff_ffff * 32'hffff_ffff, 32'h0000_0001);
		$vogls_assert_eq(32'hfad4_09e2 * 32'hb4a6_9f3c, 32'hcc61_aef8);
		$vogls_assert_eq(32'h8f97_97b0 * 32'h1ca3_c448, 32'h47d5_6980);
		$vogls_assert_eq(32'h8d72_48e2 * 32'h6e06_8097, 32'h6425_fd4e);
		$vogls_assert_eq(32'h0ab5_4bde * 32'h5b99_62c6, 32'hf0f1_a9b4);
		$vogls_assert_eq(32'hae9b_ec36 * 32'haabc_25fa, 32'h6110_7abc);
		$vogls_assert_eq(32'hdfed_2c43 * 32'hbfdd_c3d9, 32'hab98_8dcb);
		$vogls_assert_eq(32'h698c_206f * 32'hecab_3301, 32'h5227_3d6f);
		$vogls_assert_eq(32'h3f87_e362 * 32'hb386_f7a4, 32'h5abd_38c8);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(32'bz0z1_1zxx_1101_zxx0_xxzx_00x1_x00z_xxz0 * 32'b1001_0011_1001_1011_0100_0110_0010_1101, 32'bxxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx);
		$vogls_assert_eq(32'b0001_0101_1011_0101_0010_1001_0000_1000 * 32'bx0z0_1xz0_xx0x_xzxz_x11z_z1xz_0xx1_zxzz, 32'bxxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx);
		$vogls_assert_eq(32'bx01x_1110_01z0_zz1z_zz11_00z1_1z01_0z1z * 32'bxzzz_1zzx_1xz1_xz0x_1xxx_0111_z110_zzxz, 32'bxxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx);
		$vogls_assert_eq(32'bz01z_z0zz_0xxz_z1z1_xzz0_zxz1_z10z_00z1 * 32'bz10x_zx1z_xxzx_zx0z_00x1_0001_1011_1z01, 32'bxxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx);
`endif

		$vogls_assert_eq(33'h0_0000_0000 * 33'h0_0000_0000, 33'h0_0000_0000);
		$vogls_assert_eq(33'h0_0000_0000 * 33'h1_ffff_ffff, 33'h0_0000_0000);
		$vogls_assert_eq(33'h1_ffff_ffff * 33'h0_0000_0000, 33'h0_0000_0000);
		$vogls_assert_eq(33'h1_ffff_ffff * 33'h1_ffff_ffff, 33'h0_0000_0001);
		$vogls_assert_eq(33'h0_5e6f_ea07 * 33'h0_b7e6_427c, 33'h1_dbd5_2964);
		$vogls_assert_eq(33'h0_4fa0_3f26 * 33'h0_9424_aed5, 33'h0_0498_5e9e);
		$vogls_assert_eq(33'h1_edcb_8cb6 * 33'h1_6014_1de9, 33'h0_bb6b_afa6);
		$vogls_assert_eq(33'h0_32c5_bd89 * 33'h0_3e2b_6091, 33'h0_ed16_ba99);
		$vogls_assert_eq(33'h0_ce37_14af * 33'h1_0a83_81be, 33'h0_39da_88e2);
		$vogls_assert_eq(33'h1_8861_fe18 * 33'h1_a959_7663, 33'h0_175c_5348);
		$vogls_assert_eq(33'h1_a5c5_650c * 33'h0_7d7d_dbed, 33'h1_880b_d01c);
		$vogls_assert_eq(33'h1_272a_6d8e * 33'h1_a684_6099, 33'h0_e2a8_b9de);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(33'bz_zzxx_10xz_1zzx_0zx1_z1xx_xxx0_101z_z1zz * 33'b0_0000_0100_0110_1010_0000_1101_1111_0101, 33'bx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx);
		$vogls_assert_eq(33'b0_1000_1000_1011_1101_0001_0011_1101_0001 * 33'bx_1z1x_xzxz_xxzx_xx10_1x01_11zx_x01x_1x1x, 33'bx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx);
		$vogls_assert_eq(33'bx_00x1_z00x_zzzx_10xz_00zz_0011_x010_z1zz * 33'bz_xzx0_011x_0111_010z_zzx0_1xxz_01x1_z011, 33'bx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx);
		$vogls_assert_eq(33'bx_1001_xxz0_zxzx_zz00_zxx0_010x_01zz_x1zz * 33'b0_zxzx_x01x_zzxz_0z0x_xx0z_0zz0_1xzz_01x1, 33'bx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx);
`endif

		$vogls_assert_eq(63'h0000_0000_0000_0000 * 63'h0000_0000_0000_0000, 63'h0000_0000_0000_0000);
		$vogls_assert_eq(63'h0000_0000_0000_0000 * 63'h7fff_ffff_ffff_ffff, 63'h0000_0000_0000_0000);
		$vogls_assert_eq(63'h7fff_ffff_ffff_ffff * 63'h0000_0000_0000_0000, 63'h0000_0000_0000_0000);
		$vogls_assert_eq(63'h7fff_ffff_ffff_ffff * 63'h7fff_ffff_ffff_ffff, 63'h0000_0000_0000_0001);
		$vogls_assert_eq(63'h49bc_473f_ed7b_f656 * 63'h7c16_128d_b2c0_8394, 63'h345d_ef1a_4738_6bb8);
		$vogls_assert_eq(63'h0763_fcd0_1f15_c7b6 * 63'h4f8d_5238_288b_78b5, 63'h5209_9190_0fd5_83ae);
		$vogls_assert_eq(63'h0380_2b70_8d03_c91e * 63'h6872_13f9_8d60_5936, 63'h43fb_1a23_00f7_da54);
		$vogls_assert_eq(63'h3985_fb62_17dc_8eff * 63'h1d0b_c9bd_e9b5_c5cf, 63'h73ec_2708_36ac_db31);
		$vogls_assert_eq(63'h276a_a6ce_d507_55d9 * 63'h4ab7_706e_b773_50ca, 63'h0e4f_05ae_1218_8d3a);
		$vogls_assert_eq(63'h6a5d_932b_45ff_2c83 * 63'h7b85_179a_d5b0_77e0, 63'h59d6_deac_7607_d7a0);
		$vogls_assert_eq(63'h78e3_654b_faf1_4ff0 * 63'h74ef_f545_3e65_2603, 63'h7f0f_2564_6c61_8fd0);
		$vogls_assert_eq(63'h2507_4181_8d1f_b540 * 63'h30cb_d755_6232_b17a, 63'h4393_6c24_13ed_a080);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(63'b10x_zxx0_xxz1_zzxx_zzx1_1z1z_0xzz_z1z0_1x0z_0z01_z10z_xxz0_xzxz_001x_10z0_0z1x * 63'b000_0111_0110_1001_0001_0110_0101_1111_1110_0111_0100_0110_1100_1100_1011_1001, 63'bxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx);
		$vogls_assert_eq(63'b010_0001_1010_0100_0011_0100_0100_1111_1011_1011_0111_1011_1110_1110_0000_0011 * 63'b0x0_xxxz_11z1_110z_1z11_zxzx_0zx1_0zxx_zxzx_1zz0_1zxx_x0zx_00zx_1x11_x100_xz0x, 63'bxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx);
		$vogls_assert_eq(63'b0xx_z111_xx1x_zxx0_z011_zx0z_0x0z_xxzx_01z0_x10z_xz01_00xz_0011_zzxz_1z0z_zx0x * 63'b1zz_1x0x_x0zx_10z0_100x_1110_111x_10x1_1x10_010x_1x01_x01z_00zz_x0z1_0xz0_0zzz, 63'bxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx);
		$vogls_assert_eq(63'b0zz_00x1_01xx_zxzz_001z_z1zx_zzz0_xzxx_x1z0_000z_0x10_x0zx_z0xx_x011_z10x_x0x1 * 63'bz0x_01xx_xx01_1z01_001z_zzx1_xxx0_0110_0xzz_zzzx_10xz_0xzx_01z0_01x1_1xxx_00zz, 63'bxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx);
`endif

		$vogls_assert_eq(64'h0000_0000_0000_0000 * 64'h0000_0000_0000_0000, 64'h0000_0000_0000_0000);
		$vogls_assert_eq(64'h0000_0000_0000_0000 * 64'hffff_ffff_ffff_ffff, 64'h0000_0000_0000_0000);
		$vogls_assert_eq(64'hffff_ffff_ffff_ffff * 64'h0000_0000_0000_0000, 64'h0000_0000_0000_0000);
		$vogls_assert_eq(64'hffff_ffff_ffff_ffff * 64'hffff_ffff_ffff_ffff, 64'h0000_0000_0000_0001);
		$vogls_assert_eq(64'h2112_507c_2cfa_55b0 * 64'hb423_ccde_8857_5117, 64'h0aa3_5a9b_de6a_62d0);
		$vogls_assert_eq(64'hce63_22b6_ab05_347f * 64'h0ad4_5230_bdf6_6ba5, 64'hd829_6dc9_9c55_eadb);
		$vogls_assert_eq(64'hdcb3_3df3_13ee_cdc6 * 64'h4a1d_ab32_6aed_fdc7, 64'ha75e_2a9b_fc4c_a2ea);
		$vogls_assert_eq(64'h9e3d_750d_f296_d9f0 * 64'h168f_ae12_5ca2_60c9, 64'h53f4_4f94_340b_1d70);
		$vogls_assert_eq(64'h1b19_d8b8_d830_2081 * 64'h96c0_44d0_6f88_7f28, 64'hbcf9_f49a_db2d_1328);
		$vogls_assert_eq(64'h1422_373f_8622_68d1 * 64'h38b8_f24e_56ea_57b3, 64'hff9c_e458_80b8_5123);
		$vogls_assert_eq(64'h2b0a_bedd_c774_44cb * 64'ha21b_0307_82af_085c, 64'h6f1d_2be6_6cb4_10f4);
		$vogls_assert_eq(64'h828c_37e7_87d5_b7be * 64'hc6b6_e4ad_e7ea_8d5a, 64'hdd70_92c2_dd02_3ecc);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(64'bx110_1x11_101z_zzxx_1z0z_zxx0_x0xz_z00x_x00z_z0z1_xz10_z0x0_101z_z1xx_xzzx_0x0x * 64'b0100_1000_1011_1101_1101_1011_0011_1110_0110_0010_1111_0101_1101_1111_0010_1011, 64'bxxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx);
		$vogls_assert_eq(64'b1110_1111_0010_1101_1101_1100_1100_0100_1000_1101_1111_0110_0110_0001_1101_1010 * 64'b1x01_xxz1_0xzx_10zx_0xx1_1xz1_1110_x00x_1z11_11z0_zx1z_xz11_xz1x_zzxz_z111_x0zx, 64'bxxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx);
		$vogls_assert_eq(64'b0x01_xz1z_01zx_0z0z_x1z0_x10x_0x11_0xz1_zz00_0x11_z0x1_x001_zz00_z011_xz0x_1z10 * 64'bx00z_z0zx_x0x0_x10x_10xx_1zz1_10z0_x101_0xxz_zx01_xx0x_x0xz_zzzx_1zzx_x0z0_z1xx, 64'bxxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx);
		$vogls_assert_eq(64'b00xx_xzz1_zxz0_xzx0_0zx1_01z0_101x_xz01_zxxz_01xz_xx10_01zz_0xxz_1xz1_010z_1z0x * 64'bx0xx_11x1_0111_z0xx_z100_xzzz_z01x_0zzx_111z_001x_11x1_xx0x_1x1x_0111_x000_0x1z, 64'bxxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx);
`endif

		$vogls_assert_eq(65'h0_0000_0000_0000_0000 * 65'h0_0000_0000_0000_0000, 65'h0_0000_0000_0000_0000);
		$vogls_assert_eq(65'h0_0000_0000_0000_0000 * 65'h1_ffff_ffff_ffff_ffff, 65'h0_0000_0000_0000_0000);
		$vogls_assert_eq(65'h1_ffff_ffff_ffff_ffff * 65'h0_0000_0000_0000_0000, 65'h0_0000_0000_0000_0000);
		$vogls_assert_eq(65'h1_ffff_ffff_ffff_ffff * 65'h1_ffff_ffff_ffff_ffff, 65'h0_0000_0000_0000_0001);
		$vogls_assert_eq(65'h0_a394_ed54_9e3c_5a88 * 65'h1_4d56_0a3d_f4e7_069a, 65'h0_14ed_f1c3_eb25_a5d0);
		$vogls_assert_eq(65'h1_67f5_ae02_af5b_8f47 * 65'h1_2877_f5d9_0f56_75f8, 65'h1_9271_7915_0408_3fc8);
		$vogls_assert_eq(65'h1_7bf7_e1d3_6a66_2fce * 65'h1_dc5b_e7d1_fcef_1972, 65'h0_7357_0b2c_c57e_67bc);
		$vogls_assert_eq(65'h0_6612_4ab4_f9b2_1e6e * 65'h1_1b2e_dedb_8fc8_5fc0, 65'h1_efd6_2ea5_96d1_a480);
		$vogls_assert_eq(65'h0_776a_bf09_3de2_8859 * 65'h0_7318_b96d_4479_0612, 65'h0_771e_7148_bf30_ac42);
		$vogls_assert_eq(65'h0_18c8_a616_2410_8e9a * 65'h1_f204_be89_4a4b_5563, 65'h0_a8c1_8b05_bcde_478e);
		$vogls_assert_eq(65'h1_9d89_6047_dd39_d793 * 65'h0_500e_15c0_b89b_df7f, 65'h0_28a4_9f93_507b_feed);
		$vogls_assert_eq(65'h1_28ca_aa1d_c35b_ec2c * 65'h1_83c5_01cb_fef7_d9ea, 65'h1_45ab_deb6_03ab_2c38);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(65'bz_xz00_zz11_x00x_zzx0_0z1x_xzx0_z01x_x0zz_x010_zxxx_11z0_001x_zz11_xx1z_xxzx_z0xx * 65'b1_1001_0101_0110_1101_1000_0000_1110_0100_0110_1010_1010_0010_0110_0010_0001_0110, 65'bx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx);
		$vogls_assert_eq(65'b0_0010_0011_0010_0100_0101_0010_0001_0001_0100_0101_1000_1000_1111_1100_0001_1011 * 65'b0_zx10_z1z1_1001_0zxz_1z1z_z011_xzzx_1zxx_xz11_xx1x_x1zx_zxxx_0zzx_1x0x_0xzx_z11x, 65'bx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx);
		$vogls_assert_eq(65'b1_1011_00zx_z010_1zzz_1xxx_0010_1z10_zz1x_00x0_xxzz_1zx1_0x1z_x10x_x11z_zx0z_011z * 65'bx_10xz_1zxx_x0xx_100z_x1zx_0xx1_11z1_zxzz_110z_x000_111x_z01z_0z0z_1zz0_zx0z_x010, 65'bx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx);
		$vogls_assert_eq(65'b0_zx00_0z0x_z11z_xzzz_z01x_01z1_100x_zzz1_1zx1_x100_z011_000z_010z_z110_1x1x_x1x1 * 65'b1_zxx0_1zz0_xzxz_z1xz_1z11_x1z0_zz1z_1x1x_0zx0_10xz_0011_z1xx_0110_z11z_1xxz_zxxx, 65'bx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx);
`endif

		$vogls_assert_eq(127'h0000_0000_0000_0000_0000_0000_0000_0000 * 127'h0000_0000_0000_0000_0000_0000_0000_0000, 127'h0000_0000_0000_0000_0000_0000_0000_0000);
		$vogls_assert_eq(127'h0000_0000_0000_0000_0000_0000_0000_0000 * 127'h7fff_ffff_ffff_ffff_ffff_ffff_ffff_ffff, 127'h0000_0000_0000_0000_0000_0000_0000_0000);
		$vogls_assert_eq(127'h7fff_ffff_ffff_ffff_ffff_ffff_ffff_ffff * 127'h0000_0000_0000_0000_0000_0000_0000_0000, 127'h0000_0000_0000_0000_0000_0000_0000_0000);
		$vogls_assert_eq(127'h7fff_ffff_ffff_ffff_ffff_ffff_ffff_ffff * 127'h7fff_ffff_ffff_ffff_ffff_ffff_ffff_ffff, 127'h0000_0000_0000_0000_0000_0000_0000_0001);
		$vogls_assert_eq(127'h7f4d_908c_e1ff_f6c0_8347_f1da_5c11_ab6d * 127'h5ee6_8495_3400_29c3_ea11_905b_51fa_cefa, 127'h4190_76f0_723f_f2ba_0e31_a592_06a5_1e72);
		$vogls_assert_eq(127'h255b_30e2_bdc2_b74b_38a5_6b49_dd34_aa18 * 127'h6ae6_6582_b852_b3cd_9442_b5e6_9583_1d58, 127'h6795_2332_c55a_e042_ce53_e1f6_03a7_3040);
		$vogls_assert_eq(127'h5472_0d2d_3239_9ffb_2704_6f25_4b91_b8cb * 127'h41e0_2a89_566a_9ce7_af52_1b94_f186_e47a, 127'h5ad2_0982_412e_fb19_d986_2aad_a148_dcbe);
		$vogls_assert_eq(127'h7d97_ac0f_bd56_09c4_a446_1663_b93b_17fb * 127'h08a8_ac7f_8621_38ad_0f09_105c_f50d_7a37, 127'h74df_65a3_fdd2_980a_149f_99da_63de_c4ed);
		$vogls_assert_eq(127'h7ce4_0679_a8c6_dbc0_7bb3_447b_e9aa_144b * 127'h1b65_6dab_2de4_6e91_e8e6_e840_f090_f5a0, 127'h0b83_5883_c2a7_6824_51a3_5183_89e8_75e0);
		$vogls_assert_eq(127'h261e_2555_863e_1f2a_18a8_06ef_eb05_3fc4 * 127'h446f_64dd_5abc_a6e5_d0b1_e3eb_409e_7a80, 127'h040d_23ac_3644_dea3_6606_8010_5dfb_4a00);
		$vogls_assert_eq(127'h5f80_4eeb_143a_9aff_659b_0105_cb80_f288 * 127'h39d1_c7ff_b27e_1442_fd7f_e973_3e42_df5b, 127'h49c5_bfe8_7bfc_960c_4fe7_d668_212a_ae58);
		$vogls_assert_eq(127'h17ef_a621_081c_79d4_c274_37eb_a65f_7b8a * 127'h026b_5383_a301_ec03_0e5d_8c6a_79db_7862, 127'h0a68_92e4_d5e2_0111_ab28_f269_5b83_fad4);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(127'b10z_zx00_11zx_00x0_1x01_1z0x_11zx_z10x_00z1_0xzx_0xzz_1xzz_1z0x_100x_1z1z_xx00_0xzx_x010_1zzz_000x_0xz0_xzz0_11z1_1xxx_z101_zxzx_z0xx_1z1z_01x0_zxzx_zxz1_xx1z * 127'b110_0101_1111_0011_0000_0111_0111_1001_1111_0110_1110_1110_0101_1100_1101_1011_0010_1101_0111_1001_1010_0010_1101_0001_0101_0000_0110_1110_0101_0000_1100_0111, 127'bxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx);
		$vogls_assert_eq(127'b010_1110_1101_1100_0000_0001_0111_1110_0001_1101_1111_0000_0101_1101_0001_1000_0100_1110_0010_1101_0110_0010_0101_0101_0011_1101_0001_0011_0001_1010_0011_1101 * 127'bxxz_1xzx_zx01_10z1_zx0z_z1x1_x1zz_0zzz_z0x0_01x1_0zxz_000x_11zx_zz1x_0z0x_z001_xxxz_1zxz_0zzx_0x00_0x01_111x_zz1z_11zz_01zz_z011_x001_xx1z_0xzx_zz0x_xz1x_01x0, 127'bxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx);
		$vogls_assert_eq(127'bx1z_z1zz_1z11_zx11_xz0x_0z11_zzzz_zzz1_001x_0x1x_1zz0_0xx0_z1z0_1x0z_xx0z_011x_z111_10xx_1xxx_x01z_0x11_11x1_zz1x_zzz0_0zxx_zzx0_0zx0_z1z1_zxxx_x1zx_000z_001z * 127'bz0z_11x1_x0xx_0x10_zx1z_0001_z011_z00x_0z1z_01z0_00xx_x1z1_1001_zx11_zz10_010z_xzx1_1x01_x0x1_1zx0_1zz0_1z00_xx1z_zx1z_0000_x100_zxzz_z01x_0x01_1xz0_xz10_10z0, 127'bxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx);
		$vogls_assert_eq(127'bz0x_1001_01zx_x1x1_x00x_zx0x_0zzx_0z1x_1x00_1000_10zx_z110_z1x1_zxx0_x001_10z0_zxxx_0z0x_1xx1_zxz1_110z_01x0_x0zx_011z_z01x_001z_xz00_1z00_z11x_xxzx_xzx1_0110 * 127'b101_zzxz_0x1z_xzx0_01x0_1x10_1xz0_xx10_zz10_z010_0011_1zx0_x000_1z10_x00z_01zz_z0zz_1x11_xxz1_01xz_zz00_xz1z_110z_xzzx_xzx1_0x10_x0zx_1101_1x01_zx11_1xx0_x11z, 127'bxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx);
`endif

		$vogls_assert_eq(128'h0000_0000_0000_0000_0000_0000_0000_0000 * 128'h0000_0000_0000_0000_0000_0000_0000_0000, 128'h0000_0000_0000_0000_0000_0000_0000_0000);
		$vogls_assert_eq(128'h0000_0000_0000_0000_0000_0000_0000_0000 * 128'hffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff, 128'h0000_0000_0000_0000_0000_0000_0000_0000);
		$vogls_assert_eq(128'hffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff * 128'h0000_0000_0000_0000_0000_0000_0000_0000, 128'h0000_0000_0000_0000_0000_0000_0000_0000);
		$vogls_assert_eq(128'hffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff * 128'hffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff, 128'h0000_0000_0000_0000_0000_0000_0000_0001);
		$vogls_assert_eq(128'h7d8d_3ddd_2cf1_49cb_4f1e_1a37_8ac8_ad93 * 128'h9a06_9295_3b8d_3447_0e61_065f_171b_1b80, 128'h80a1_8e3b_bd46_fab1_8061_7db8_1226_4a80);
		$vogls_assert_eq(128'h002c_cb25_7a5d_7c82_86c5_547c_051a_0039 * 128'hce73_34f2_3393_2b6c_9d37_efb6_eafb_3e90, 128'hbf4a_7f0c_1c94_b776_b3a8_b337_7c90_ee10);
		$vogls_assert_eq(128'h0f3f_0f48_52d6_b779_de43_79cb_d823_0076 * 128'heacf_33a9_b466_fab6_962c_04dd_25d0_7461, 128'hafad_21b7_aad1_a1d8_911f_6d8c_2f58_a4b6);
		$vogls_assert_eq(128'h9605_3035_6545_c030_a981_fa54_f121_e129 * 128'ha630_ad9d_7cb9_23f5_f5ed_f382_a3a6_ef83, 128'ha051_5af0_0678_f3ef_cab8_96bd_2121_7efb);
		$vogls_assert_eq(128'hc978_4b52_9d1c_0838_edc8_cca8_e3bc_d6f3 * 128'hd080_12b8_96c5_6fe7_aa60_4892_3926_12e9, 128'hbb41_85d8_04b9_d61a_7d33_ee34_910e_b92b);
		$vogls_assert_eq(128'hb041_6f19_bbe7_1835_4e8b_f78a_60dd_c7f1 * 128'h9146_8cdc_f716_4b73_c555_23ed_fe89_56c8, 128'h10a1_7298_01f6_11a7_ff58_d0d4_4c68_2a48);
		$vogls_assert_eq(128'h2401_342a_7f05_fbf5_efca_c577_1aa6_f7cb * 128'h7256_56e3_55f3_ad50_e2fb_b984_bab2_7bce, 128'hfd11_adaf_acfc_0325_c96b_6f73_748f_ee5a);
		$vogls_assert_eq(128'hb173_fcdf_3fe1_e8f3_35ad_de88_43e4_f668 * 128'he271_fff9_2a95_27f6_f817_0ebf_6b6e_64c5, 128'h0448_f9e9_50c7_0470_dd4c_e059_0822_3e08);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(128'bzzxz_0z11_z00x_0xx0_0x1z_100x_z0z1_z100_x11z_xxx1_00zz_10x0_z0xz_1001_1x0z_0xzz_zx0z_1xzx_1z0x_zzx1_x010_0xz1_0zzz_1zx0_z010_z111_110z_0z11_xx00_zzxx_z0z0_z110 * 128'b1100_0111_1001_0000_0011_1101_1100_0101_1001_0000_0000_1100_0000_1010_1111_0111_0001_1011_0000_1010_1110_0010_0010_1110_1110_1100_1100_1000_0001_0111_1010_0110, 128'bxxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx);
		$vogls_assert_eq(128'b0100_1011_1010_1100_1011_1110_1101_1111_1111_1111_0101_1010_0010_0010_1001_1110_0111_1111_0101_0011_1101_0011_0100_0100_1010_0111_1100_1000_0101_0100_1110_0100 * 128'bxxzx_zzz0_1xz1_z0zx_10zx_x0xx_xxz0_11xz_xxz0_1z1z_zx01_0xz1_0111_010z_0x1x_xzz0_01xz_z1zx_x1x0_x0z0_1111_1xz0_0z11_01xx_xzzx_zx1z_xz00_xxx0_z0xx_xxxx_1110_zx10, 128'bxxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx);
		$vogls_assert_eq(128'bzzz0_1x1z_xx11_x0z1_x1x0_1x1x_zxxx_xz0z_00z1_0zxz_0111_xzz1_x10z_x1x0_0zx1_11z0_1zzz_zzz0_01x0_x100_11xx_10zx_100x_xzxz_1xzx_11z0_1111_010x_0zx0_0000_0zzz_0xz1 * 128'b10xz_0x1z_00x0_0z11_x1xz_x10z_1zx1_0000_z100_0x1z_xxz0_010x_xxz0_0001_1xxz_x0xx_10zz_zxzz_z101_010z_zz1z_x01z_z0zz_00xx_01x0_z1zx_z100_1zz1_0zx1_xx01_zz10_zzz0, 128'bxxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx);
		$vogls_assert_eq(128'bzzx0_z00z_z000_x1zx_x01z_xzxz_0010_xxxx_001z_1z0x_110z_1z1x_0z00_zz1z_00zz_1110_zxzz_1xzx_xz11_zxx1_11xx_11x0_xzzx_z000_z100_1x1x_x10x_10xz_01z1_x0zz_zx00_x1z1 * 128'bx11x_z101_10z0_zxx1_1x0x_z101_x0xz_x0x1_101z_z0x1_1xxx_x1z0_0xz1_10xz_0zxx_0xzz_0x1x_0010_011z_01z0_10zx_0000_1x01_xzzz_zzx0_z000_00x0_xx0x_zzz0_z111_zx1x_x010, 128'bxxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx);
`endif

		$vogls_assert_eq(129'h0_0000_0000_0000_0000_0000_0000_0000_0000 * 129'h0_0000_0000_0000_0000_0000_0000_0000_0000, 129'h0_0000_0000_0000_0000_0000_0000_0000_0000);
		$vogls_assert_eq(129'h0_0000_0000_0000_0000_0000_0000_0000_0000 * 129'h1_ffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff, 129'h0_0000_0000_0000_0000_0000_0000_0000_0000);
		$vogls_assert_eq(129'h1_ffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff * 129'h0_0000_0000_0000_0000_0000_0000_0000_0000, 129'h0_0000_0000_0000_0000_0000_0000_0000_0000);
		$vogls_assert_eq(129'h1_ffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff * 129'h1_ffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff, 129'h0_0000_0000_0000_0000_0000_0000_0000_0001);
		$vogls_assert_eq(129'h1_98be_5ff0_929d_af9f_0149_a2af_0ab1_a504 * 129'h1_15ca_dfaf_f26d_3dd3_8c2a_a0b1_8982_cbf8, 129'h0_0624_6659_dd0d_06e7_9b90_7e56_29fa_07e0);
		$vogls_assert_eq(129'h0_2934_4e15_2477_f047_8336_a758_d9f9_3502 * 129'h0_40b3_af7c_d5b2_11a9_9dec_2b83_3885_8f7e, 129'h1_e621_32c7_7a32_cc23_fb7e_3794_774e_34fc);
		$vogls_assert_eq(129'h0_abf3_9096_4a68_369c_12b9_9a50_8cb6_d1f0 * 129'h0_5b6d_6b7c_54b2_a582_f398_1e9e_7cb8_82a0, 129'h1_b741_0b1f_ad2a_cf1b_9e5a_bd03_ed5f_1600);
		$vogls_assert_eq(129'h1_199c_8d94_3c0a_584e_a7cd_13cb_abda_06b2 * 129'h1_3189_1f48_e66e_2c11_11de_febf_9e98_961d, 129'h1_50c5_78c4_e2f9_b003_cb7e_c928_0d4f_0e2a);
		$vogls_assert_eq(129'h0_502e_d818_013c_a998_2b2b_f58c_6da5_8dad * 129'h0_5c07_4653_d711_73e4_46d4_7d41_6fa5_d3b0, 129'h1_b75d_ead0_0831_c81c_5f23_74fa_2917_fdf0);
		$vogls_assert_eq(129'h1_e0bd_202c_54e1_4f3a_bba4_1ae0_adf8_f767 * 129'h1_c657_20ae_5bd6_7d4d_7202_fae5_767d_3b27, 129'h0_b639_f17b_4ed0_42af_a635_7a10_293d_6db1);
		$vogls_assert_eq(129'h0_89e2_df24_e42c_0aea_23d7_263d_d391_d017 * 129'h0_a06a_187d_e2e6_e7a6_bbc9_9473_4363_589c, 129'h0_5a9f_2580_6058_8ab3_4a65_4f95_8a47_b604);
		$vogls_assert_eq(129'h0_970c_9eb4_c4ea_4ece_67b6_e467_9432_d797 * 129'h0_647a_c591_5da6_4a3a_260c_cfb4_c673_5380, 129'h1_dcef_8706_4887_b4e6_b3cd_5d98_3826_c080);
`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(129'bx_1z0z_x011_1z11_1zx0_1z10_xx10_1xzz_z00x_0xx1_zzx0_xz1z_xxzz_01z1_10x1_111x_01zx_0zzz_11z0_1011_1010_1x1z_0x01_zzz1_1z11_x1xz_01zz_1001_z101_z000_z10z_zx0z_1zzx * 129'b1_0001_1001_0000_0011_1111_1001_1011_0011_1001_0011_0111_1100_1010_0111_0111_1100_1011_0010_0101_1011_0110_1101_1110_0111_1101_0110_0101_0010_0001_1110_0000_1000, 129'bx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx);
		$vogls_assert_eq(129'b1_0101_0000_0101_1001_1111_0111_0111_1011_0111_1001_1111_1111_1000_1001_1101_1001_0010_1101_1011_0001_0011_1101_1110_1011_1110_1110_0110_1111_1111_1011_1110_1010 * 129'b1_xz10_xzx1_0000_z1xz_x01z_xxzx_xz1z_z1xz_xxxz_101z_101z_x01z_001z_z00x_1x00_zzx0_1zz0_1x10_110x_1010_xzxz_z100_0xxx_x11z_z0x1_z00x_x0xx_zzz1_xzz0_z0x0_z11x_zx0x, 129'bx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx);
		$vogls_assert_eq(129'b1_x11z_111x_1111_x100_0z00_zxxz_xz1x_z110_zz0x_z1z0_zx00_1z0x_xx1x_1xzx_x1zz_1zxx_zxz1_110x_1zz1_11z0_1zz1_xzxx_0z01_01xz_0zxx_xxxx_0z0x_11xx_xzz0_zxz1_1xzz_001x * 129'b1_xxxx_zxzx_zz11_011x_011z_000z_00z0_0x10_zx11_xz11_100x_xzzx_z11z_zxxz_10zx_xzx1_x001_xxz0_zzx0_zx10_x1zz_z11z_z1x0_z1xx_xz10_11x0_zzxz_1x01_0010_xx1z_0101_0z1x, 129'bx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx);
		$vogls_assert_eq(129'bz_xzxz_0x01_0xz0_z11x_z001_zz10_1z01_zzxx_100x_zx1x_0101_zx01_x0z1_00z1_x011_z0x0_0x10_1z1x_xz10_zx1x_0zzx_z11x_xzx1_01zx_xxz0_z10x_1x0z_1z1x_xzx0_0001_1010_z100 * 129'bz_zx0x_1001_z0z0_xzx0_x00x_0100_1z11_11z0_xzz0_11zx_1zxx_xxzz_x11z_xxzz_z00x_01x0_x11x_0x10_0zx1_1xx1_1xx0_z10x_z1z0_x0z0_0xxz_z010_z0x0_x1x1_10zz_01zx_z11x_0z0z, 129'bx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx);
`endif
	end
endmodule
