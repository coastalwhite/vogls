module tb();
    initial begin
        $vogls_assert_eq(1'b0 & 1'b0, 1'b0);
        $vogls_assert_eq(1'b1 & 1'b0, 1'b0);
        $vogls_assert_eq(1'b0 & 1'b1, 1'b0);
        $vogls_assert_eq(1'b1 & 1'b1, 1'b1);
`ifndef __VOGLS__TWO_VALUE_LOGIC
        $vogls_assert_eq(1'bx & 1'b0, 1'b0);
        $vogls_assert_eq(1'bx & 1'b1, 1'bx);
        $vogls_assert_eq(1'bx & 1'bx, 1'bx);
        $vogls_assert_eq(1'bx & 1'bz, 1'bx);

        $vogls_assert_eq(1'bz & 1'b0, 1'b0);
        $vogls_assert_eq(1'bz & 1'b1, 1'bx);
        $vogls_assert_eq(1'bz & 1'bx, 1'bx);
        $vogls_assert_eq(1'bz & 1'bz, 1'bx);

        $vogls_assert_eq(1'b0 & 1'bx, 1'b0);
        $vogls_assert_eq(1'b1 & 1'bx, 1'bx);
        $vogls_assert_eq(1'b0 & 1'bz, 1'b0);
        $vogls_assert_eq(1'b1 & 1'bz, 1'bx);
`endif

        $vogls_assert_eq(1'b0 | 1'b0, 1'b0);
        $vogls_assert_eq(1'b1 | 1'b0, 1'b1);
        $vogls_assert_eq(1'b0 | 1'b1, 1'b1);
        $vogls_assert_eq(1'b1 | 1'b1, 1'b1);
`ifndef __VOGLS__TWO_VALUE_LOGIC
        $vogls_assert_eq(1'bx | 1'b0, 1'bx);
        $vogls_assert_eq(1'bx | 1'b1, 1'b1);
        $vogls_assert_eq(1'bx | 1'bx, 1'bx);
        $vogls_assert_eq(1'bx | 1'bz, 1'bx);

        $vogls_assert_eq(1'bz | 1'b0, 1'bx);
        $vogls_assert_eq(1'bz | 1'b1, 1'b1);
        $vogls_assert_eq(1'bz | 1'bx, 1'bx);
        $vogls_assert_eq(1'bz | 1'bz, 1'bx);

        $vogls_assert_eq(1'b0 | 1'bx, 1'bx);
        $vogls_assert_eq(1'b1 | 1'bx, 1'b1);
        $vogls_assert_eq(1'b0 | 1'bz, 1'bx);
        $vogls_assert_eq(1'b1 | 1'bz, 1'b1);
`endif

        $vogls_assert_eq(1'b0 ^ 1'b0, 1'b0);
        $vogls_assert_eq(1'b1 ^ 1'b0, 1'b1);
        $vogls_assert_eq(1'b0 ^ 1'b1, 1'b1);
        $vogls_assert_eq(1'b1 ^ 1'b1, 1'b0);
`ifndef __VOGLS__TWO_VALUE_LOGIC
        $vogls_assert_eq(1'bx ^ 1'b0, 1'bx);
        $vogls_assert_eq(1'bx ^ 1'b1, 1'bx);
        $vogls_assert_eq(1'bx ^ 1'bx, 1'bx);
        $vogls_assert_eq(1'bx ^ 1'bz, 1'bx);

        $vogls_assert_eq(1'bz ^ 1'b0, 1'bx);
        $vogls_assert_eq(1'bz ^ 1'b1, 1'bx);
        $vogls_assert_eq(1'bz ^ 1'bx, 1'bx);
        $vogls_assert_eq(1'bz ^ 1'bz, 1'bx);

        $vogls_assert_eq(1'b0 ^ 1'bx, 1'bx);
        $vogls_assert_eq(1'b1 ^ 1'bx, 1'bx);
        $vogls_assert_eq(1'b0 ^ 1'bz, 1'bx);
        $vogls_assert_eq(1'b1 ^ 1'bz, 1'bx);
`endif

		// 7-bit
		$vogls_assert_eq(7'h00 & 7'h00, 7'h00);
		$vogls_assert_eq(7'h00 & 7'h7f, 7'h00);
		$vogls_assert_eq(7'h7f & 7'h00, 7'h00);
		$vogls_assert_eq(7'h7f & 7'h7f, 7'h7f);
		$vogls_assert_eq(7'h01 & 7'h0e, 7'h00);
		$vogls_assert_eq(7'h70 & 7'h74, 7'h70);
		$vogls_assert_eq(7'h47 & 7'h0f, 7'h07);
		$vogls_assert_eq(7'h75 & 7'h02, 7'h00);

		$vogls_assert_eq(7'h00 | 7'h00, 7'h00);
		$vogls_assert_eq(7'h00 | 7'h7f, 7'h7f);
		$vogls_assert_eq(7'h7f | 7'h00, 7'h7f);
		$vogls_assert_eq(7'h7f | 7'h7f, 7'h7f);
		$vogls_assert_eq(7'h1c | 7'h1c, 7'h1c);
		$vogls_assert_eq(7'h6e | 7'h0f, 7'h6f);
		$vogls_assert_eq(7'h55 | 7'h4c, 7'h5d);
		$vogls_assert_eq(7'h73 | 7'h08, 7'h7b);

		$vogls_assert_eq(7'h00 ^ 7'h00, 7'h00);
		$vogls_assert_eq(7'h00 ^ 7'h7f, 7'h7f);
		$vogls_assert_eq(7'h7f ^ 7'h00, 7'h7f);
		$vogls_assert_eq(7'h7f ^ 7'h7f, 7'h00);
		$vogls_assert_eq(7'h6a ^ 7'h60, 7'h0a);
		$vogls_assert_eq(7'h21 ^ 7'h28, 7'h09);
		$vogls_assert_eq(7'h13 ^ 7'h65, 7'h76);
		$vogls_assert_eq(7'h25 ^ 7'h2a, 7'h0f);

`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(7'bzx0_11x1 & 7'bxz0_110x, 7'bxx0_110x);
		$vogls_assert_eq(7'b00z_x00x & 7'b0z0_x1zz, 7'b000_x00x);

		$vogls_assert_eq(7'b0xz_z111 | 7'b1zz_0x10, 7'b1xx_x111);
		$vogls_assert_eq(7'b001_1xxx | 7'b00z_z1zx, 7'b001_11xx);

		$vogls_assert_eq(7'b1x0_x1x1 ^ 7'b1zz_x0xz, 7'b0xx_x1xx);
		$vogls_assert_eq(7'bx01_0xz1 ^ 7'bzzx_z1xz, 7'bxxx_xxxx);
`endif

		// 31-bit
		$vogls_assert_eq(31'h0000_0000 & 31'h0000_0000, 31'h0000_0000);
		$vogls_assert_eq(31'h0000_0000 & 31'h7fff_ffff, 31'h0000_0000);
		$vogls_assert_eq(31'h7fff_ffff & 31'h0000_0000, 31'h0000_0000);
		$vogls_assert_eq(31'h7fff_ffff & 31'h7fff_ffff, 31'h7fff_ffff);
		$vogls_assert_eq(31'h56a9_42f6 & 31'h0ddb_e430, 31'h0489_4030);
		$vogls_assert_eq(31'h1eaa_9840 & 31'h5705_f613, 31'h1600_9000);
		$vogls_assert_eq(31'h09cf_e080 & 31'h14fe_2d62, 31'h00ce_2000);
		$vogls_assert_eq(31'h2a4a_89ef & 31'h7322_5ec3, 31'h2202_08c3);

		$vogls_assert_eq(31'h0000_0000 | 31'h0000_0000, 31'h0000_0000);
		$vogls_assert_eq(31'h0000_0000 | 31'h7fff_ffff, 31'h7fff_ffff);
		$vogls_assert_eq(31'h7fff_ffff | 31'h0000_0000, 31'h7fff_ffff);
		$vogls_assert_eq(31'h7fff_ffff | 31'h7fff_ffff, 31'h7fff_ffff);
		$vogls_assert_eq(31'h7f61_44cf | 31'h1b4c_bd51, 31'h7f6d_fddf);
		$vogls_assert_eq(31'h7c9c_d891 | 31'h3b6c_6d66, 31'h7ffc_fdf7);
		$vogls_assert_eq(31'h2812_39f0 | 31'h2b21_f81b, 31'h2b33_f9fb);
		$vogls_assert_eq(31'h36d0_0fc4 | 31'h569a_e86b, 31'h76da_efef);

		$vogls_assert_eq(31'h0000_0000 ^ 31'h0000_0000, 31'h0000_0000);
		$vogls_assert_eq(31'h0000_0000 ^ 31'h7fff_ffff, 31'h7fff_ffff);
		$vogls_assert_eq(31'h7fff_ffff ^ 31'h0000_0000, 31'h7fff_ffff);
		$vogls_assert_eq(31'h7fff_ffff ^ 31'h7fff_ffff, 31'h0000_0000);
		$vogls_assert_eq(31'h75c2_0cd0 ^ 31'h5138_92fc, 31'h24fa_9e2c);
		$vogls_assert_eq(31'h1c9e_6d1c ^ 31'h0487_39f4, 31'h1819_54e8);
		$vogls_assert_eq(31'h1419_36da ^ 31'h263a_fdfb, 31'h3223_cb21);
		$vogls_assert_eq(31'h193b_267d ^ 31'h68e9_cee5, 31'h71d2_e898);

`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(31'bx11_xx1z_x0zx_1xz1_x101_xzzz_zx01_z101 & 31'bx10_0xx1_0zzz_xz01_xx1z_0001_0zzx_0z1x, 31'bx10_0xxx_00xx_xx01_xx0x_000x_0x0x_0x0x);
		$vogls_assert_eq(31'bz11_1x0z_11zx_010z_01z0_1xx1_zzx0_1011 & 31'b10z_10xz_0x1z_1x01_000x_zzz0_xxxz_xz11, 31'bx0x_100x_0xxx_0x0x_0000_xxx0_xxx0_x011);

		$vogls_assert_eq(31'bzzx_zzzx_z1xx_z010_1111_111x_0xx1_00xz | 31'bx1z_1xx0_zxxz_xz00_xxz0_11x1_0x00_zzz0, 31'bx1x_1xxx_x1xx_xx10_1111_1111_0xx1_xxxx);
		$vogls_assert_eq(31'bx10_x01z_zx01_1xxx_x1x1_z1x1_0x00_001z | 31'bxz1_z1zx_xxxx_01zz_001x_1100_01x0_100z, 31'bx11_x11x_xxx1_11xx_x111_11x1_01x0_101x);

		$vogls_assert_eq(31'bz0z_zxzx_10x0_x1xx_zz10_zzzz_zx10_0z0x ^ 31'bxxz_zx10_x101_01z1_1100_0011_000z_0x0x, 31'bxxx_xxxx_x1x1_x0xx_xx10_xxxx_xx1x_0x0x);
		$vogls_assert_eq(31'bx10_0zz0_x11z_0x0z_0z11_z10x_1100_0xxz ^ 31'b10x_xz1x_zzx1_zxx0_z011_0xz1_x10z_1xz0, 31'bx1x_xxxx_xxxx_xxxx_xx00_xxxx_x00x_1xxx);
`endif

		// 32-bit
		$vogls_assert_eq(32'h0000_0000 & 32'h0000_0000, 32'h0000_0000);
		$vogls_assert_eq(32'h0000_0000 & 32'hffff_ffff, 32'h0000_0000);
		$vogls_assert_eq(32'hffff_ffff & 32'h0000_0000, 32'h0000_0000);
		$vogls_assert_eq(32'hffff_ffff & 32'hffff_ffff, 32'hffff_ffff);
		$vogls_assert_eq(32'h3787_d610 & 32'h7394_73e2, 32'h3384_5200);
		$vogls_assert_eq(32'hb373_9306 & 32'hce2d_d09e, 32'h8221_9006);
		$vogls_assert_eq(32'hdab2_7da4 & 32'hf4a6_2f8b, 32'hd0a2_2d80);
		$vogls_assert_eq(32'h78ef_e25f & 32'h168f_a3fe, 32'h108f_a25e);

		$vogls_assert_eq(32'h0000_0000 | 32'h0000_0000, 32'h0000_0000);
		$vogls_assert_eq(32'h0000_0000 | 32'hffff_ffff, 32'hffff_ffff);
		$vogls_assert_eq(32'hffff_ffff | 32'h0000_0000, 32'hffff_ffff);
		$vogls_assert_eq(32'hffff_ffff | 32'hffff_ffff, 32'hffff_ffff);
		$vogls_assert_eq(32'h71c7_b784 | 32'h5890_7ee9, 32'h79d7_ffed);
		$vogls_assert_eq(32'hb55f_e7fb | 32'h454b_db90, 32'hf55f_fffb);
		$vogls_assert_eq(32'hbdb6_5639 | 32'hd130_ec65, 32'hfdb6_fe7d);
		$vogls_assert_eq(32'h44c6_9563 | 32'h0187_cc12, 32'h45c7_dd73);

		$vogls_assert_eq(32'h0000_0000 ^ 32'h0000_0000, 32'h0000_0000);
		$vogls_assert_eq(32'h0000_0000 ^ 32'hffff_ffff, 32'hffff_ffff);
		$vogls_assert_eq(32'hffff_ffff ^ 32'h0000_0000, 32'hffff_ffff);
		$vogls_assert_eq(32'hffff_ffff ^ 32'hffff_ffff, 32'h0000_0000);
		$vogls_assert_eq(32'h8fdf_2939 ^ 32'heb9d_1a4e, 32'h6442_3377);
		$vogls_assert_eq(32'hbdf5_bd7b ^ 32'h8a5a_ad9f, 32'h37af_10e4);
		$vogls_assert_eq(32'h66d8_aaf2 ^ 32'hb82d_42a6, 32'hdef5_e854);
		$vogls_assert_eq(32'hb37e_3f6f ^ 32'h8886_5b7b, 32'h3bf8_6414);

`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(32'b1xx1_xz1z_000z_0xz0_xz00_z1x1_10xz_zzxz & 32'b0z01_0x01_x10x_0xzx_1001_x0x1_0zz1_zzx1, 32'b0x01_0x0x_000x_0xx0_x000_x0x1_00xx_xxxx);
		$vogls_assert_eq(32'b1z0z_0z0x_1x0z_0x01_x0xz_01x1_001x_1z11 & 32'b11zx_z1zx_00z1_010z_xz0x_1zx1_zxx0_0xx1, 32'b1x0x_0x0x_000x_0x0x_x00x_0xx1_00x0_0xx1);

		$vogls_assert_eq(32'bz0zx_z0z1_x1x1_x0zz_0xxx_xxx1_0z0z_z1zx | 32'bzx1x_zzz0_x11x_1xx0_011z_xz1z_1xzz_zzz0, 32'bxx1x_xxx1_x111_1xxx_011x_xx11_1xxx_x1xx);
		$vogls_assert_eq(32'bx1x1_1x10_1x00_001z_0x1x_z010_zz0x_0xxz | 32'bx10x_z0zz_xx1x_x111_x0zz_1zz1_x0z0_z010, 32'bx1x1_1x1x_1x1x_x111_xx1x_1x11_xxxx_xx1x);

		$vogls_assert_eq(32'bxx1z_x0xz_zxx1_0xzx_1xz0_xx10_zz0z_1zz0 ^ 32'b110z_0z1x_00z0_zx1z_xzz0_1010_00zz_xxz1, 32'bxx1x_xxxx_xxx1_xxxx_xxx0_xx00_xxxx_xxx1);
		$vogls_assert_eq(32'b010z_x1z1_z01z_zz1z_zz10_xx0z_01xx_111x ^ 32'bx1zz_zx10_1101_z1xz_x0z0_x1z1_zz0x_001z, 32'bx0xx_xxx1_x11x_xxxx_xxx0_xxxx_xxxx_110x);
`endif

		// 33-bit
		$vogls_assert_eq(33'h00_0000_0000 & 33'h00_0000_0000, 33'h00_0000_0000);
		$vogls_assert_eq(33'h00_0000_0000 & 33'h01_ffff_ffff, 33'h00_0000_0000);
		$vogls_assert_eq(33'h01_ffff_ffff & 33'h00_0000_0000, 33'h00_0000_0000);
		$vogls_assert_eq(33'h01_ffff_ffff & 33'h01_ffff_ffff, 33'h01_ffff_ffff);
		$vogls_assert_eq(33'h00_2157_e8e1 & 33'h01_ab08_2114, 33'h00_2100_2000);
		$vogls_assert_eq(33'h01_8acb_0b89 & 33'h00_4a47_53ca, 33'h00_0a43_0388);
		$vogls_assert_eq(33'h00_ae0c_809f & 33'h01_fe87_c8d4, 33'h00_ae04_8094);
		$vogls_assert_eq(33'h00_ffe6_7315 & 33'h00_93d0_96ef, 33'h00_93c0_1205);

		$vogls_assert_eq(33'h00_0000_0000 | 33'h00_0000_0000, 33'h00_0000_0000);
		$vogls_assert_eq(33'h00_0000_0000 | 33'h01_ffff_ffff, 33'h01_ffff_ffff);
		$vogls_assert_eq(33'h01_ffff_ffff | 33'h00_0000_0000, 33'h01_ffff_ffff);
		$vogls_assert_eq(33'h01_ffff_ffff | 33'h01_ffff_ffff, 33'h01_ffff_ffff);
		$vogls_assert_eq(33'h01_a4bd_5170 | 33'h00_555a_386c, 33'h01_f5ff_797c);
		$vogls_assert_eq(33'h00_c233_8840 | 33'h00_6d4a_3cae, 33'h00_ef7b_bcee);
		$vogls_assert_eq(33'h01_f586_ef9e | 33'h00_6f58_9dc5, 33'h01_ffde_ffdf);
		$vogls_assert_eq(33'h01_1023_a96a | 33'h01_dcf7_8c6f, 33'h01_dcf7_ad6f);

		$vogls_assert_eq(33'h00_0000_0000 ^ 33'h00_0000_0000, 33'h00_0000_0000);
		$vogls_assert_eq(33'h00_0000_0000 ^ 33'h01_ffff_ffff, 33'h01_ffff_ffff);
		$vogls_assert_eq(33'h01_ffff_ffff ^ 33'h00_0000_0000, 33'h01_ffff_ffff);
		$vogls_assert_eq(33'h01_ffff_ffff ^ 33'h01_ffff_ffff, 33'h00_0000_0000);
		$vogls_assert_eq(33'h00_fc0a_a887 ^ 33'h00_22d5_1a77, 33'h00_dedf_b2f0);
		$vogls_assert_eq(33'h00_0b07_a80c ^ 33'h01_75f8_3403, 33'h01_7eff_9c0f);
		$vogls_assert_eq(33'h01_596b_7f33 ^ 33'h01_df66_c27a, 33'h00_860d_bd49);
		$vogls_assert_eq(33'h01_0a37_c12d ^ 33'h00_b352_969e, 33'h01_b965_57b3);

`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(33'b0_x010_1z01_1x01_z111_1011_x00x_xx01_xz1z & 33'bz_xz0z_z010_00xz_z1z0_zz00_zx00_xz1z_zx11, 33'b0_x000_x000_000x_x1x0_x000_x000_xx0x_xx1x);
		$vogls_assert_eq(33'bz_10xz_xx01_xz00_z1x1_zzx0_11xz_zzzz_10zx & 33'b1_1111_010z_xx0x_1xzx_1z1x_0xxx_zz10_zzzx, 33'bx_10xx_0x0x_xx00_xxxx_xxx0_0xxx_xxx0_x0xx);

		$vogls_assert_eq(33'bz_01xx_100z_0111_0zzz_1011_z01z_x1zx_x100 | 33'bz_z0xx_0100_10z1_1110_zzxx_x00z_01zx_11zz, 33'bx_x1xx_110x_1111_111x_1x11_x01x_x1xx_11xx);
		$vogls_assert_eq(33'bz_001x_1x0z_xzx0_z1z1_0zz0_0xzz_0111_1xz1 | 33'bz_11zz_1xz1_11zz_zx00_1z0x_zzz1_zz0x_0xxx, 33'bx_111x_1xx1_11xx_x1x1_1xxx_xxx1_x111_1xx1);

		$vogls_assert_eq(33'bx_z011_x00x_x1x1_11xz_0xzz_zzx0_1z10_001z ^ 33'b0_00xz_zz0x_00z0_101z_zx00_x011_01z1_xzx1, 33'bx_x0xx_xx0x_x1x1_01xx_xxxx_xxx1_1xx1_xxxx);
		$vogls_assert_eq(33'bz_x0z1_zz10_10x1_z0z1_zx1x_z1z0_1xxx_1z1x ^ 33'b1_1xxz_z0xx_xzzx_z10x_1xzz_011x_0xx0_x100, 33'bx_xxxx_xxxx_xxxx_x1xx_xxxx_x0xx_1xxx_xx1x);
`endif

		// 63-bit
		$vogls_assert_eq(63'h0000_0000_0000_0000 & 63'h0000_0000_0000_0000, 63'h0000_0000_0000_0000);
		$vogls_assert_eq(63'h0000_0000_0000_0000 & 63'h7fff_ffff_ffff_ffff, 63'h0000_0000_0000_0000);
		$vogls_assert_eq(63'h7fff_ffff_ffff_ffff & 63'h0000_0000_0000_0000, 63'h0000_0000_0000_0000);
		$vogls_assert_eq(63'h7fff_ffff_ffff_ffff & 63'h7fff_ffff_ffff_ffff, 63'h7fff_ffff_ffff_ffff);
		$vogls_assert_eq(63'h37ca_e692_ad21_714c & 63'h2414_6c13_3b0b_ca53, 63'h2400_6412_2901_4040);
		$vogls_assert_eq(63'h0103_e70f_3db1_5c5d & 63'h6a98_a34e_ad76_bd6a, 63'h0000_a30e_2d30_1c48);
		$vogls_assert_eq(63'h1fab_6319_edd5_6bc4 & 63'h255f_ca68_e656_1010, 63'h050b_4208_e454_0000);
		$vogls_assert_eq(63'h3aac_74dc_944b_3e3e & 63'h3aae_0d99_ce3c_1b79, 63'h3aac_0498_8408_1a38);

		$vogls_assert_eq(63'h0000_0000_0000_0000 | 63'h0000_0000_0000_0000, 63'h0000_0000_0000_0000);
		$vogls_assert_eq(63'h0000_0000_0000_0000 | 63'h7fff_ffff_ffff_ffff, 63'h7fff_ffff_ffff_ffff);
		$vogls_assert_eq(63'h7fff_ffff_ffff_ffff | 63'h0000_0000_0000_0000, 63'h7fff_ffff_ffff_ffff);
		$vogls_assert_eq(63'h7fff_ffff_ffff_ffff | 63'h7fff_ffff_ffff_ffff, 63'h7fff_ffff_ffff_ffff);
		$vogls_assert_eq(63'h51cf_b458_8c42_9118 | 63'h3875_9706_d206_0c88, 63'h79ff_b75e_de46_9d98);
		$vogls_assert_eq(63'h4bbb_cb23_e343_389d | 63'h6347_a6c7_b707_d41a, 63'h6bff_efe7_f747_fc9f);
		$vogls_assert_eq(63'h71bc_93ad_dbcc_0627 | 63'h1179_dde4_552a_d535, 63'h71fd_dfed_dfee_d737);
		$vogls_assert_eq(63'h37a2_78df_bd75_d79f | 63'h290e_0cdf_4fdb_027f, 63'h3fae_7cdf_ffff_d7ff);

		$vogls_assert_eq(63'h0000_0000_0000_0000 ^ 63'h0000_0000_0000_0000, 63'h0000_0000_0000_0000);
		$vogls_assert_eq(63'h0000_0000_0000_0000 ^ 63'h7fff_ffff_ffff_ffff, 63'h7fff_ffff_ffff_ffff);
		$vogls_assert_eq(63'h7fff_ffff_ffff_ffff ^ 63'h0000_0000_0000_0000, 63'h7fff_ffff_ffff_ffff);
		$vogls_assert_eq(63'h7fff_ffff_ffff_ffff ^ 63'h7fff_ffff_ffff_ffff, 63'h0000_0000_0000_0000);
		$vogls_assert_eq(63'h5299_6fef_3723_66ac ^ 63'h1906_3dcd_3699_fe71, 63'h4b9f_5222_01ba_98dd);
		$vogls_assert_eq(63'h30c1_95b2_e89d_bb20 ^ 63'h390b_897c_28d9_e161, 63'h09ca_1cce_c044_5a41);
		$vogls_assert_eq(63'h6fb8_eb91_41da_f87b ^ 63'h1d25_ce77_7d7b_d88b, 63'h729d_25e6_3ca1_20f0);
		$vogls_assert_eq(63'h7969_ecfd_f8dd_7e1b ^ 63'h17c2_a714_14ed_11a8, 63'h6eab_4be9_ec30_6fb3);

`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(63'b0zx_z1xx_111x_xzx1_xz0x_0110_0z10_x0x1_1xz1_0xz1_x11x_z10x_z1zx_xz1x_111x_xz0x & 63'bxx0_zx00_00zz_xxz1_z0z1_10zz_xz0x_100z_0000_001z_10zx_zx10_xx00_1xxz_zzx1_0zz1, 63'b0x0_xx00_00xx_xxx1_x00x_00x0_0x00_x00x_0000_00xx_x0xx_xx00_xx00_xxxx_xxxx_0x0x);
		$vogls_assert_eq(63'b1xx_00z1_z11x_11x1_1xzx_0zx1_zxx0_0xz0_xzzx_zz0x_110x_1110_xz0z_z101_xx1x_101x & 63'b1zz_11zz_1zzz_010x_0111_x1zx_1011_0x1z_00z1_xzzx_xzzx_0x1x_zzzx_zx0x_1xx0_0010, 63'b1xx_00xx_xxxx_010x_0xxx_0xxx_x0x0_0xx0_00xx_xx0x_xx0x_0x10_xx0x_xx0x_xxx0_0010);

		$vogls_assert_eq(63'b0xz_xx1x_xx0x_xzzz_xzzz_z1z1_0zx0_1zz1_0x1x_1101_x1z0_11z1_x010_z011_0zx1_1xz1 | 63'b0zz_z1x0_x0x1_0101_z100_1x1x_101z_xzz1_z10x_1xx1_x1zx_00zx_1100_zzxz_xz1x_xz01, 63'b0xx_x11x_xxx1_x1x1_x1xx_1111_1x1x_1xx1_x11x_11x1_x1xx_11x1_1110_xx11_xx11_1xx1);
		$vogls_assert_eq(63'b001_00x1_xz0x_1z1x_xzxz_0z00_zx10_001x_z1x1_111x_x1x1_zz1z_x0z1_00xz_x0z1_1011 | 63'bzxz_10z0_zz0x_00xz_0x11_11xz_10x1_zz10_01z1_xz0z_110x_01z1_x00x_001x_100x_xzx0, 63'bxx1_10x1_xx0x_1x1x_xx11_11xx_1x11_xx1x_x1x1_111x_11x1_x111_x0x1_001x_10x1_1x11);

		$vogls_assert_eq(63'b0x0_zxz0_11xx_1zxz_zzx0_x0zx_1z00_0z11_1zz1_xxz0_11x1_1z1x_zxzx_z1x0_01xx_zxx1 ^ 63'b10x_1x0z_00x1_1110_x00z_xx01_01zz_0x0x_xx01_xx00_zx0z_xx0z_1000_xzx1_011z_xzxz, 63'b1xx_xxxx_11xx_0xxx_xxxx_xxxx_1xxx_0x1x_xxx0_xxx0_xxxx_xx1x_xxxx_xxx1_00xx_xxxx);
		$vogls_assert_eq(63'bzzz_11z0_0110_110x_11z0_11z1_0zzz_x11z_01xx_xxz1_x0xx_110z_1z11_111z_0z0z_z010 ^ 63'bz01_0zz0_z01z_xxz0_10zz_1zzx_zz1z_zz10_11xz_1zz1_zz1x_xzzx_z000_0zz1_x111_z11z, 63'bxxx_1xx0_x10x_xxxx_01xx_0xxx_xxxx_xx0x_10xx_xxx0_xxxx_xxxx_xx11_1xxx_xx1x_x10x);
`endif

		// 64-bit
		$vogls_assert_eq(64'h0000_0000_0000_0000 & 64'h0000_0000_0000_0000, 64'h0000_0000_0000_0000);
		$vogls_assert_eq(64'h0000_0000_0000_0000 & 64'hffff_ffff_ffff_ffff, 64'h0000_0000_0000_0000);
		$vogls_assert_eq(64'hffff_ffff_ffff_ffff & 64'h0000_0000_0000_0000, 64'h0000_0000_0000_0000);
		$vogls_assert_eq(64'hffff_ffff_ffff_ffff & 64'hffff_ffff_ffff_ffff, 64'hffff_ffff_ffff_ffff);
		$vogls_assert_eq(64'h3875_ee5f_5f37_5aed & 64'h313e_0dd0_cf2e_b1c9, 64'h3034_0c50_4f26_10c9);
		$vogls_assert_eq(64'h5d95_bd43_9062_2868 & 64'h1876_b89f_6127_d2d3, 64'h1814_b803_0022_0040);
		$vogls_assert_eq(64'h496c_54bc_ea56_27c5 & 64'h4acf_6af5_e424_8623, 64'h484c_40b4_e004_0601);
		$vogls_assert_eq(64'hd839_2d4c_c0ce_eab6 & 64'hb91e_67a2_ef64_312a, 64'h9818_2500_c044_2022);

		$vogls_assert_eq(64'h0000_0000_0000_0000 | 64'h0000_0000_0000_0000, 64'h0000_0000_0000_0000);
		$vogls_assert_eq(64'h0000_0000_0000_0000 | 64'hffff_ffff_ffff_ffff, 64'hffff_ffff_ffff_ffff);
		$vogls_assert_eq(64'hffff_ffff_ffff_ffff | 64'h0000_0000_0000_0000, 64'hffff_ffff_ffff_ffff);
		$vogls_assert_eq(64'hffff_ffff_ffff_ffff | 64'hffff_ffff_ffff_ffff, 64'hffff_ffff_ffff_ffff);
		$vogls_assert_eq(64'h7dd0_7f4f_078f_7f4a | 64'h25e7_6cce_590b_5a2a, 64'h7df7_7fcf_5f8f_7f6a);
		$vogls_assert_eq(64'h8af6_c241_6ca2_df83 | 64'h896d_d0bc_4e91_29b9, 64'h8bff_d2fd_6eb3_ffbb);
		$vogls_assert_eq(64'hc8bb_d878_7d87_74b4 | 64'h949c_fea0_b681_8ad6, 64'hdcbf_fef8_ff87_fef6);
		$vogls_assert_eq(64'h78d4_a144_911d_d1bc | 64'h5a79_7b50_da84_a034, 64'h7afd_fb54_db9d_f1bc);

		$vogls_assert_eq(64'h0000_0000_0000_0000 ^ 64'h0000_0000_0000_0000, 64'h0000_0000_0000_0000);
		$vogls_assert_eq(64'h0000_0000_0000_0000 ^ 64'hffff_ffff_ffff_ffff, 64'hffff_ffff_ffff_ffff);
		$vogls_assert_eq(64'hffff_ffff_ffff_ffff ^ 64'h0000_0000_0000_0000, 64'hffff_ffff_ffff_ffff);
		$vogls_assert_eq(64'hffff_ffff_ffff_ffff ^ 64'hffff_ffff_ffff_ffff, 64'h0000_0000_0000_0000);
		$vogls_assert_eq(64'h7556_7b11_0373_06be ^ 64'he457_eedc_4b25_ef1d, 64'h9101_95cd_4856_e9a3);
		$vogls_assert_eq(64'hf4b2_7f61_ba8f_afd0 ^ 64'hed62_13ff_3045_f3bb, 64'h19d0_6c9e_8aca_5c6b);
		$vogls_assert_eq(64'h0525_590b_6466_9e94 ^ 64'hd273_2742_c1c9_6b1e, 64'hd756_7e49_a5af_f58a);
		$vogls_assert_eq(64'h27fb_c90b_2c91_66f7 ^ 64'h7970_c08b_bdf7_7587, 64'h5e8b_0980_9166_1370);

`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(64'bxxz0_zxx1_0xx1_0zz0_01x0_x1z1_zxz0_x1x1_00xz_10zz_x0z1_1001_0zzz_x0z1_0x1z_xzzz & 64'bxxz1_zxz1_1zx1_111z_100x_z001_01xz_x1z0_z010_zz0z_z11z_00x0_01zz_xx1x_1zxz_0z0z, 64'bxxx0_xxx1_0xx1_0xx0_0000_x001_0xx0_x1x0_00x0_x00x_x0xx_0000_0xxx_x0xx_0xxx_0x0x);
		$vogls_assert_eq(64'b1010_0xx1_11x0_x0z1_00x0_x10z_11z1_z0x1_0z00_zx10_z0x1_0xx1_1z1z_xzzx_1x11_z110 & 64'b1110_xx0z_1xz0_x010_1zzz_11x1_1x1z_xx01_xx00_1z01_xx01_0z0z_10z1_010z_11z0_zx01, 64'b1010_0x0x_1xx0_x0x0_00x0_x10x_1xxx_x001_0x00_xx00_x001_0x0x_10xx_0x0x_1xx0_xx00);

		$vogls_assert_eq(64'b11xx_0zxz_z00z_1zx1_0x1x_1xxx_1x1x_xx11_0xxx_x00z_0100_z1z1_00xz_zzx0_zxx0_z1z1 | 64'bzz10_1x1z_0xzz_xzzx_z01z_101z_zxx1_000x_zxzz_xzzz_z110_10zx_1zzx_1000_0111_zzzz, 64'b111x_1x1x_xxxx_1xx1_xx1x_1x1x_1x11_xx11_xxxx_xxxx_x110_11x1_1xxx_1xx0_x111_x1x1);
		$vogls_assert_eq(64'b10zx_101z_0zz0_x1zz_x11x_1x1x_0xzx_1z10_0x1x_1zx1_1001_x0z0_10zz_1xzz_0101_111x | 64'bxz1z_001z_10z1_011x_zxx1_x00x_zz00_100x_101z_x110_xx0z_xzxx_0010_000x_z010_xz00, 64'b1x1x_101x_1xx1_x11x_x111_1x1x_xxxx_1x1x_1x1x_1111_1x01_xxxx_101x_1xxx_x111_111x);

		$vogls_assert_eq(64'bz0z1_zzx0_z100_1z1z_1x01_xzxz_z01z_xx1x_zzxz_1xxx_10z1_00z0_z1x0_x010_0x10_z1xx ^ 64'b0x0z_0110_0xx1_1xzx_0000_1xz1_zzzx_0xz0_xx10_z1zx_zzxz_1zz1_z01x_z0xz_zz01_z00x, 64'bxxxx_xxx0_xxx1_0xxx_1x01_xxxx_xxxx_xxxx_xxxx_xxxx_xxxx_1xx1_x1xx_x0xx_xx11_x1xx);
		$vogls_assert_eq(64'b0x10_z1xz_01xz_10xz_1111_zxxx_z1x0_00z0_xxx0_1xx1_01z1_x1zx_z101_z1xz_xxz1_xzxx ^ 64'b0x10_z11x_1xx1_zx0x_1000_11xz_01z0_zzz1_zz10_zxzz_1xzz_z1x0_10x1_1z1x_xzx0_xxx0, 64'b0x00_x0xx_1xxx_xxxx_0111_xxxx_x0x0_xxx1_xxx0_xxxx_1xxx_x0xx_x1x0_xxxx_xxx1_xxxx);
`endif

		// 65-bit
		$vogls_assert_eq(65'h00_0000_0000_0000_0000 & 65'h00_0000_0000_0000_0000, 65'h00_0000_0000_0000_0000);
		$vogls_assert_eq(65'h00_0000_0000_0000_0000 & 65'h01_ffff_ffff_ffff_ffff, 65'h00_0000_0000_0000_0000);
		$vogls_assert_eq(65'h01_ffff_ffff_ffff_ffff & 65'h00_0000_0000_0000_0000, 65'h00_0000_0000_0000_0000);
		$vogls_assert_eq(65'h01_ffff_ffff_ffff_ffff & 65'h01_ffff_ffff_ffff_ffff, 65'h01_ffff_ffff_ffff_ffff);
		$vogls_assert_eq(65'h00_fb1c_b3c7_7967_7c83 & 65'h01_4763_2103_7845_89b5, 65'h00_4300_2103_7845_0881);
		$vogls_assert_eq(65'h00_20c2_2452_0829_f19a & 65'h00_e7be_09f9_50a0_68f2, 65'h00_2082_0050_0020_6092);
		$vogls_assert_eq(65'h00_5da2_b541_e325_1628 & 65'h01_f671_a924_e6c5_652e, 65'h00_5420_a100_e205_0428);
		$vogls_assert_eq(65'h01_9569_12c3_4ddd_63e6 & 65'h01_67c6_b1c0_df35_cc37, 65'h01_0540_10c0_4d15_4026);

		$vogls_assert_eq(65'h00_0000_0000_0000_0000 | 65'h00_0000_0000_0000_0000, 65'h00_0000_0000_0000_0000);
		$vogls_assert_eq(65'h00_0000_0000_0000_0000 | 65'h01_ffff_ffff_ffff_ffff, 65'h01_ffff_ffff_ffff_ffff);
		$vogls_assert_eq(65'h01_ffff_ffff_ffff_ffff | 65'h00_0000_0000_0000_0000, 65'h01_ffff_ffff_ffff_ffff);
		$vogls_assert_eq(65'h01_ffff_ffff_ffff_ffff | 65'h01_ffff_ffff_ffff_ffff, 65'h01_ffff_ffff_ffff_ffff);
		$vogls_assert_eq(65'h01_e009_2a95_b60a_0852 | 65'h00_0851_1b2a_0592_69e2, 65'h01_e859_3bbf_b79a_69f2);
		$vogls_assert_eq(65'h01_d2df_fda3_5cc2_2393 | 65'h00_39fe_376b_be2f_92de, 65'h01_fbff_ffeb_feef_b3df);
		$vogls_assert_eq(65'h01_fb07_5fb9_2b4b_8079 | 65'h00_81fe_66ce_56ec_79c2, 65'h01_fbff_7fff_7fef_f9fb);
		$vogls_assert_eq(65'h00_3e25_604a_c8ef_4f75 | 65'h00_962a_df17_320d_7ec7, 65'h00_be2f_ff5f_faef_7ff7);

		$vogls_assert_eq(65'h00_0000_0000_0000_0000 ^ 65'h00_0000_0000_0000_0000, 65'h00_0000_0000_0000_0000);
		$vogls_assert_eq(65'h00_0000_0000_0000_0000 ^ 65'h01_ffff_ffff_ffff_ffff, 65'h01_ffff_ffff_ffff_ffff);
		$vogls_assert_eq(65'h01_ffff_ffff_ffff_ffff ^ 65'h00_0000_0000_0000_0000, 65'h01_ffff_ffff_ffff_ffff);
		$vogls_assert_eq(65'h01_ffff_ffff_ffff_ffff ^ 65'h01_ffff_ffff_ffff_ffff, 65'h00_0000_0000_0000_0000);
		$vogls_assert_eq(65'h01_3f64_0d3d_f2b6_42a1 ^ 65'h01_f39d_440a_8bab_350c, 65'h00_ccf9_4937_791d_77ad);
		$vogls_assert_eq(65'h01_4b89_4462_b828_b04b ^ 65'h00_76ab_7e6f_a234_0440, 65'h01_3d22_3a0d_1a1c_b40b);
		$vogls_assert_eq(65'h01_cada_2f8b_376a_ab39 ^ 65'h00_dd82_00a3_4089_b286, 65'h01_1758_2f28_77e3_19bf);
		$vogls_assert_eq(65'h01_92e3_e796_b140_1944 ^ 65'h00_549b_d42e_d562_3421, 65'h01_c678_33b8_6422_2d65);

`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(65'bz_111x_10xx_xx11_1x01_xxxz_1111_0xx1_0xzz_z0xz_101z_0x11_10xz_xz0z_0011_z1z0_011z & 65'bx_zz0x_xz11_0zzx_z010_z1x1_1x11_1x1z_00x1_zxz0_100z_z10z_0z1z_011z_x11x_z0x0_z00z, 65'bx_xx0x_x0xx_0xxx_x000_xxxx_1x11_0xxx_00xx_x0x0_100x_0x0x_00xx_0x0x_001x_x0x0_000x);
		$vogls_assert_eq(65'bz_1110_x0zx_zzxx_001x_zz0z_xz10_1011_0xz1_xxxz_xx10_x1z0_0xzx_1zxx_zxzx_x1zz_1xz1 & 65'bz_zx0x_xzzz_x100_1x1z_zxz0_x11z_x00z_xx01_1010_0xzx_zx00_0xxx_1zz1_x1zz_10xx_zx0z, 65'bx_xx00_x0xx_xx00_001x_xx00_xx10_x00x_0x01_x0x0_0xx0_xx00_0xxx_1xxx_xxxx_x0xx_xx0x);

		$vogls_assert_eq(65'bx_z0xx_z11z_xzzz_1111_x1x0_zzxx_x110_1xzz_xxx0_011x_0011_0z0z_1x00_1x1x_00xx_zzxx | 65'b1_xxx1_z11z_zz1x_00x1_z11x_xz00_1xzx_zx0x_z1zx_0z1x_x110_x0zz_x10z_00zz_11x0_x100, 65'b1_xxx1_x11x_xx1x_1111_x11x_xxxx_111x_1xxx_x1xx_011x_x111_xxxx_110x_1x1x_11xx_x1xx);
		$vogls_assert_eq(65'b0_01xx_11x1_xz11_x001_1zz0_x1z0_z1x0_0xxx_zz10_10x1_0x01_zx00_0zzx_10x0_10zx_1100 | 65'bz_0110_z0x0_1x1x_1x00_z1xx_x0zz_11zx_010x_z011_1zz0_zxz0_00xx_z0z1_zxxx_1x0x_x11z, 65'bx_011x_11x1_1x11_1x01_11xx_x1xx_11xx_01xx_xx11_1xx1_xxx1_xxxx_xxx1_1xxx_1xxx_111x);

		$vogls_assert_eq(65'b1_zxxx_xx1x_00zx_1x1x_1zxz_010x_z0z0_0111_111z_10zx_xx01_z011_1x01_z01x_10zz_0z01 ^ 65'b1_x1x1_0zx1_zx10_zzxz_xz10_x11z_1xxz_01z1_010z_011x_000x_zx0z_1zzx_10x0_1110_10xz, 65'b0_xxxx_xxxx_xxxx_xxxx_xxxx_x01x_xxxx_00x0_101x_11xx_xx0x_xx1x_0xxx_x0xx_01xx_1xxx);
		$vogls_assert_eq(65'b0_010z_00zz_011z_111z_0xxz_10x1_z0zz_1z01_01z0_xz01_0z0z_0x0z_xzx0_00xz_01x0_zxxx ^ 65'b0_zz0x_xzzx_x1zz_xzxz_xx00_00x1_0x1x_01x0_1z00_zzxx_1xxx_x000_1z1z_xz11_zx10_xz0x, 65'b0_xx0x_xxxx_x0xx_xxxx_xxxx_10x0_xxxx_1xx1_1xx0_xxxx_1xxx_xx0x_xxxx_xxxx_xxx0_xxxx);
`endif

		// 127-bit
		$vogls_assert_eq(127'h0000_0000_0000_0000_0000_0000_0000_0000 & 127'h0000_0000_0000_0000_0000_0000_0000_0000, 127'h0000_0000_0000_0000_0000_0000_0000_0000);
		$vogls_assert_eq(127'h0000_0000_0000_0000_0000_0000_0000_0000 & 127'h7fff_ffff_ffff_ffff_ffff_ffff_ffff_ffff, 127'h0000_0000_0000_0000_0000_0000_0000_0000);
		$vogls_assert_eq(127'h7fff_ffff_ffff_ffff_ffff_ffff_ffff_ffff & 127'h0000_0000_0000_0000_0000_0000_0000_0000, 127'h0000_0000_0000_0000_0000_0000_0000_0000);
		$vogls_assert_eq(127'h7fff_ffff_ffff_ffff_ffff_ffff_ffff_ffff & 127'h7fff_ffff_ffff_ffff_ffff_ffff_ffff_ffff, 127'h7fff_ffff_ffff_ffff_ffff_ffff_ffff_ffff);
		$vogls_assert_eq(127'h2b17_f088_e0cc_f27e_6dc8_5a4a_8f73_bf04 & 127'h7874_72ac_beb1_76ad_d21c_aef9_eac6_4e2a, 127'h2814_7088_a080_722c_4008_0a48_8a42_0e00);
		$vogls_assert_eq(127'h63e0_db7a_fd03_ecb2_22d1_7ff5_678e_dfca & 127'h1041_9f4a_a637_5799_8428_13f7_1a8c_8dd0, 127'h0040_9b4a_a403_4490_0000_13f5_028c_8dc0);
		$vogls_assert_eq(127'h06c0_aafd_d6f3_6c80_3755_eb0c_9e79_f024 & 127'h4d7a_b356_c76a_d45c_d729_6d0f_ceac_b0c4, 127'h0440_a254_c662_4400_1701_690c_8e28_b004);
		$vogls_assert_eq(127'h10e7_4d8d_3753_6a96_eab9_8f54_b9c5_b89d & 127'h53c5_3487_163f_b213_5c0c_7b8c_d654_1372, 127'h10c5_0485_1613_2212_4808_0b04_9044_1010);

		$vogls_assert_eq(127'h0000_0000_0000_0000_0000_0000_0000_0000 | 127'h0000_0000_0000_0000_0000_0000_0000_0000, 127'h0000_0000_0000_0000_0000_0000_0000_0000);
		$vogls_assert_eq(127'h0000_0000_0000_0000_0000_0000_0000_0000 | 127'h7fff_ffff_ffff_ffff_ffff_ffff_ffff_ffff, 127'h7fff_ffff_ffff_ffff_ffff_ffff_ffff_ffff);
		$vogls_assert_eq(127'h7fff_ffff_ffff_ffff_ffff_ffff_ffff_ffff | 127'h0000_0000_0000_0000_0000_0000_0000_0000, 127'h7fff_ffff_ffff_ffff_ffff_ffff_ffff_ffff);
		$vogls_assert_eq(127'h7fff_ffff_ffff_ffff_ffff_ffff_ffff_ffff | 127'h7fff_ffff_ffff_ffff_ffff_ffff_ffff_ffff, 127'h7fff_ffff_ffff_ffff_ffff_ffff_ffff_ffff);
		$vogls_assert_eq(127'h5bbb_12e7_2308_4409_5b95_f5f6_f674_bafc | 127'h0617_18e9_949a_bf12_95ad_ff70_ac73_9439, 127'h5fbf_1aef_b79a_ff1b_dfbd_fff6_fe77_befd);
		$vogls_assert_eq(127'h017c_932b_443f_b037_c957_5fca_cb55_23bf | 127'h2bae_0774_3a14_f3b9_599a_e2e2_d89f_1ca6, 127'h2bfe_977f_7e3f_f3bf_d9df_ffea_dbdf_3fbf);
		$vogls_assert_eq(127'h1ba9_e5d6_4096_c264_53d5_f82d_2fa2_6d23 | 127'h613d_603a_e961_f8fe_29bf_a714_888b_a9f0, 127'h7bbd_e5fe_e9f7_fafe_7bff_ff3d_afab_edf3);
		$vogls_assert_eq(127'h0c54_bdc3_5dc1_bc1f_4a2d_548d_cb42_f415 | 127'h1622_e84a_4799_b0c3_97ef_3bef_bbc0_2a10, 127'h1e76_fdcb_5fd9_bcdf_dfef_7fef_fbc2_fe15);

		$vogls_assert_eq(127'h0000_0000_0000_0000_0000_0000_0000_0000 ^ 127'h0000_0000_0000_0000_0000_0000_0000_0000, 127'h0000_0000_0000_0000_0000_0000_0000_0000);
		$vogls_assert_eq(127'h0000_0000_0000_0000_0000_0000_0000_0000 ^ 127'h7fff_ffff_ffff_ffff_ffff_ffff_ffff_ffff, 127'h7fff_ffff_ffff_ffff_ffff_ffff_ffff_ffff);
		$vogls_assert_eq(127'h7fff_ffff_ffff_ffff_ffff_ffff_ffff_ffff ^ 127'h0000_0000_0000_0000_0000_0000_0000_0000, 127'h7fff_ffff_ffff_ffff_ffff_ffff_ffff_ffff);
		$vogls_assert_eq(127'h7fff_ffff_ffff_ffff_ffff_ffff_ffff_ffff ^ 127'h7fff_ffff_ffff_ffff_ffff_ffff_ffff_ffff, 127'h0000_0000_0000_0000_0000_0000_0000_0000);
		$vogls_assert_eq(127'h5551_a205_0678_e67e_a1d0_7868_5680_fda5 ^ 127'h4791_3517_78bd_fd94_b22f_31d2_55ed_1f63, 127'h12c0_9712_7ec5_1bea_13ff_49ba_036d_e2c6);
		$vogls_assert_eq(127'h2de1_3fc7_79e5_5de3_31e3_c58c_a408_a768 ^ 127'h1d6f_9f0b_db8f_2f16_e1d6_a5ce_d4f8_0ce2, 127'h308e_a0cc_a26a_72f5_d035_6042_70f0_ab8a);
		$vogls_assert_eq(127'h0c65_62c6_581f_e834_116e_0dc5_bc67_90dc ^ 127'h6866_b402_2cb8_24aa_b043_e0cd_205b_4c99, 127'h6403_d6c4_74a7_cc9e_a12d_ed08_9c3c_dc45);
		$vogls_assert_eq(127'h54ac_dc57_bf6c_e86e_1e1b_ee3e_44f7_8725 ^ 127'h3986_1d94_463e_0db0_881b_47a1_2f21_3efb, 127'h6d2a_c1c3_f952_e5de_9600_a99f_6bd6_b9de);

`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(127'b000_0z00_zz1z_x111_011x_zzzx_zz0z_000z_1z01_z111_z1xx_1xxx_z1xz_1z0z_x1z1_zzzz_z110_0zxx_11xz_z100_1z1x_xz0x_011x_11xx_x1x1_x0xx_x0z0_z0z1_1z00_0xx1_00z1_x00z & 127'bz11_000z_1zz0_0100_10xx_1010_1zz0_xx1x_zx11_01xx_00zz_0xzz_1zx0_000x_0z1x_10xx_010x_0z10_01zz_z0xx_z1zz_0100_x1zz_00zz_x11z_0x11_xx1x_x0z1_x1zz_0z1z_1101_zxz1, 127'b000_0000_xxx0_0100_00xx_x0x0_xx00_000x_xx01_01xx_00xx_0xxx_xxx0_000x_0xxx_x0xx_0100_0xx0_01xx_x000_xxxx_0x00_01xx_00xx_x1xx_00xx_x0x0_x0x1_xx00_0xxx_0001_x00x);
		$vogls_assert_eq(127'bzxx_z1z0_1xz1_zx00_xzzz_00xx_11z0_z1x1_0z01_zzx0_11zz_1z0x_x00z_0z1x_1xxz_10x1_z0xz_xx1z_z1x1_1111_xzxx_00zx_xxz1_0x0z_x0z1_011z_z11x_x101_0110_xzzz_00z1_0zx1 & 127'bzzx_x1xz_0zxz_0z1x_z00z_zzx0_z0xz_xxxz_x11x_1zz1_x0xz_zzx0_zxx0_zx1z_1x1z_01xz_zxz0_xx1x_000z_z100_101x_zz0z_zxxz_xz00_xxzx_x1xx_x0x1_zx01_1z1z_0001_xx1z_zx1x, 127'bxxx_x1x0_0xxx_0x00_x00x_00x0_x0x0_xxxx_0x0x_xxx0_x0xx_xx00_x000_0x1x_1xxx_00xx_x0x0_xx1x_000x_x100_x0xx_000x_xxxx_0x00_x0xx_01xx_x0xx_xx01_0x10_000x_00xx_0xxx);

		$vogls_assert_eq(127'bz0z_001z_z00x_1010_zxz0_zx0z_010x_1zxx_xx01_00x0_xz10_z011_zzx1_0xzz_xx1z_1xz0_1zx0_z01z_zxzz_zxx0_1x00_10xx_000x_z100_01z1_11x0_100x_z01z_z111_x0xx_xxz1_1101 | 127'bzx0_1110_xx0z_0xzx_0zx0_1111_zzzz_xzzz_xzxx_1zzx_x1z0_xzx0_xx1z_00zz_x0z1_zx10_1x0x_z111_0101_z000_1zzz_1xx1_11z1_0zzx_zzxz_1100_zx11_1011_zz01_1zzz_0110_1x10, 127'bxxx_111x_xx0x_1x1x_xxx0_1111_x1xx_1xxx_xxx1_1xxx_x110_xx11_xx11_0xxx_xx11_1x10_1xxx_x111_x1x1_xxx0_1xxx_1xx1_11x1_x1xx_x1x1_11x0_1x11_1011_x111_1xxx_x111_1111);
		$vogls_assert_eq(127'b1zz_01z1_x10z_x011_1zx0_zx00_xz10_001z_0xzz_0xzz_001x_zx1x_z1x1_0011_xz01_0z0z_x010_x1z0_x0x1_x1zx_x001_z0x1_x1xz_1z1x_x111_xz11_0111_0zzz_001z_101x_zx01_1110 | 127'b10x_z0xx_x10x_1zx1_0x0x_z1x0_101x_zxx0_0z10_x1zz_10xx_0z1z_z0x1_0x11_zxz0_0010_0xx1_z1zx_z0zx_xxz1_zzzx_x1z1_1x10_11z1_1z01_xx0x_zz01_010x_z000_0zx0_10z0_z00z, 127'b1xx_x1x1_x10x_1x11_1xxx_x1x0_1x1x_xx1x_0x1x_x1xx_101x_xx1x_x1x1_0x11_xxx1_0x1x_xx11_x1xx_x0x1_x1x1_xxx1_x1x1_111x_1111_1111_xx11_x111_01xx_x01x_1x1x_1xx1_111x);

		$vogls_assert_eq(127'b1z0_011z_xxz0_x11z_110z_1x0z_01zx_zzz0_x0z0_1zzx_1011_zxxx_xzx0_01zx_z10z_0z0x_0z0z_110x_xzxx_0z1z_1101_10x0_zzz1_zxxx_1xxx_01z0_z0zz_1x0z_x10x_0010_1001_1z0z ^ 127'bz1z_zz1x_11x1_x0z1_01xz_00zx_x1x0_101x_1000_z10z_xz1z_01xz_00x1_11x0_zx1x_0xz1_zxz0_1101_zxzz_1xz1_0x00_zz0z_110x_1xx1_xzxz_z1zz_xxxx_zz01_1z10_x1z0_1zzz_x011, 127'bxxx_xx0x_xxx1_x1xx_10xx_1xxx_x0xx_xxxx_x0x0_xxxx_xx0x_xxxx_xxx1_10xx_xx1x_0xxx_xxxx_000x_xxxx_1xxx_1x01_xxxx_xxxx_xxxx_xxxx_x0xx_xxxx_xx0x_xx1x_x1x0_0xxx_xx1x);
		$vogls_assert_eq(127'bzz0_z11x_zzzz_z10z_xxzx_010z_xx10_0zx0_10x1_1z11_0100_x1zx_111z_101z_zxz0_zz00_xz10_x110_0z00_xzz1_1z11_0x1x_x1zx_1zx1_x0z0_x1x1_x001_xz1x_x0zx_00xx_x0xz_xzz1 ^ 127'bz1x_1x01_x0x1_z11x_11xx_0zxz_xz1z_11zx_0z0x_1zzx_x01z_z01z_zzz1_x1z0_z011_100z_1z11_0xx1_10z0_xxxx_100x_1zzx_1011_x0x1_1x10_0x0x_zx01_00x1_11zx_zxxz_1xxx_0x0x, 127'bxxx_xx1x_xxxx_x01x_xxxx_0xxx_xx0x_1xxx_1xxx_0xxx_x11x_x1xx_xxxx_x1xx_xxx1_xx0x_xx01_xxx1_1xx0_xxxx_0x1x_1xxx_x1xx_xxx0_xxx0_xxxx_xx00_xxxx_x1xx_xxxx_xxxx_xxxx);
`endif

		// 128-bit
		$vogls_assert_eq(128'h0000_0000_0000_0000_0000_0000_0000_0000 & 128'h0000_0000_0000_0000_0000_0000_0000_0000, 128'h0000_0000_0000_0000_0000_0000_0000_0000);
		$vogls_assert_eq(128'h0000_0000_0000_0000_0000_0000_0000_0000 & 128'hffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff, 128'h0000_0000_0000_0000_0000_0000_0000_0000);
		$vogls_assert_eq(128'hffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff & 128'h0000_0000_0000_0000_0000_0000_0000_0000, 128'h0000_0000_0000_0000_0000_0000_0000_0000);
		$vogls_assert_eq(128'hffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff & 128'hffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff, 128'hffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff);
		$vogls_assert_eq(128'hfcef_ed5e_ee27_bdda_41c2_be73_7017_45be & 128'h587d_8eb3_07f1_9533_0cb9_9f4c_a465_fbaf, 128'h586d_8c12_0621_9512_0080_9e40_2005_41ae);
		$vogls_assert_eq(128'h8669_3300_e0db_cbb1_d5cb_e417_7aaf_dbfc & 128'hfac3_1477_e960_83d1_4469_e913_da09_8676, 128'h8241_1000_e040_8391_4449_e013_5a09_8274);
		$vogls_assert_eq(128'h5c0b_49a3_a220_382a_f575_34b7_283f_49e8 & 128'hc847_0b94_783c_ff30_18ce_02ab_1fba_4b4f, 128'h4803_0980_2020_3820_1044_00a3_083a_4948);
		$vogls_assert_eq(128'h8efa_4dc8_265f_88f6_5020_bdd6_1c89_66e8 & 128'he5b2_b802_dd6a_97ac_5ff6_7855_a20f_3ef1, 128'h84b2_0800_044a_80a4_5020_3854_0009_26e0);

		$vogls_assert_eq(128'h0000_0000_0000_0000_0000_0000_0000_0000 | 128'h0000_0000_0000_0000_0000_0000_0000_0000, 128'h0000_0000_0000_0000_0000_0000_0000_0000);
		$vogls_assert_eq(128'h0000_0000_0000_0000_0000_0000_0000_0000 | 128'hffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff, 128'hffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff);
		$vogls_assert_eq(128'hffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff | 128'h0000_0000_0000_0000_0000_0000_0000_0000, 128'hffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff);
		$vogls_assert_eq(128'hffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff | 128'hffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff, 128'hffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff);
		$vogls_assert_eq(128'hd649_2163_19be_e495_731e_419f_3cfb_5cbf | 128'h0d66_7404_0d10_8abf_9e68_9ba9_d1f7_9090, 128'hdf6f_7567_1dbe_eebf_ff7e_dbbf_fdff_dcbf);
		$vogls_assert_eq(128'hba20_a25d_e483_206a_3e18_f131_fe89_c266 | 128'hdaa2_7714_205e_883b_a398_248b_1fe4_3333, 128'hfaa2_f75d_e4df_a87b_bf98_f5bb_ffed_f377);
		$vogls_assert_eq(128'hf60c_1412_a31f_a6e2_ba41_a778_074b_2b57 | 128'h2900_2e64_7e04_053d_ff1b_bc84_26ef_38e5, 128'hff0c_3e76_ff1f_a7ff_ff5b_bffc_27ef_3bf7);
		$vogls_assert_eq(128'he43b_c8e4_c8f2_c892_6b43_28ef_5f10_7f2e | 128'hf23c_094a_1b39_1547_67d5_eb27_84c6_929a, 128'hf63f_c9ee_dbfb_ddd7_6fd7_ebef_dfd6_ffbe);

		$vogls_assert_eq(128'h0000_0000_0000_0000_0000_0000_0000_0000 ^ 128'h0000_0000_0000_0000_0000_0000_0000_0000, 128'h0000_0000_0000_0000_0000_0000_0000_0000);
		$vogls_assert_eq(128'h0000_0000_0000_0000_0000_0000_0000_0000 ^ 128'hffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff, 128'hffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff);
		$vogls_assert_eq(128'hffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff ^ 128'h0000_0000_0000_0000_0000_0000_0000_0000, 128'hffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff);
		$vogls_assert_eq(128'hffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff ^ 128'hffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff, 128'h0000_0000_0000_0000_0000_0000_0000_0000);
		$vogls_assert_eq(128'hf704_9fc0_40f1_4618_cbaa_80ca_6c1b_6b93 ^ 128'h2e08_13d0_6516_798a_3a25_124d_99b0_5a89, 128'hd90c_8c10_25e7_3f92_f18f_9287_f5ab_311a);
		$vogls_assert_eq(128'h901f_d362_adf9_f8c1_5fb5_90e6_b312_834a ^ 128'h744c_2835_66c4_13e4_f170_8438_3b0a_8aeb, 128'he453_fb57_cb3d_eb25_aec5_14de_8818_09a1);
		$vogls_assert_eq(128'hb6fe_2a5b_127b_b437_a69c_7147_40df_726e ^ 128'h3145_f1f0_f8e7_86f2_d695_0dd2_bf0d_772f, 128'h87bb_dbab_ea9c_32c5_7009_7c95_ffd2_0541);
		$vogls_assert_eq(128'he83f_4c0c_09e7_1705_b071_74c8_8496_06f7 ^ 128'h0826_d07f_030a_e759_9535_221e_84e2_7bf5, 128'he019_9c73_0aed_f05c_2544_56d6_0074_7d02);

`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(128'bz0x0_z1z0_1zz1_01xz_1z1z_zz1x_0z1x_xz10_x001_x10x_zz1x_xxz0_01z1_1xx0_z0z0_x10z_0xxz_z00x_xz01_0z1z_0001_x01x_1z10_00x0_11zz_xz1x_zxx1_x100_zxz1_x1zz_z0xx_10z0 & 128'b0xzx_zz1z_0zx1_x0x1_z001_xz1x_0z0x_10z1_z0x0_z000_x11x_z0z0_x0x0_1xx1_1100_1xzz_1x11_z011_0zzz_xx1x_00xz_0x11_10zz_z1x1_zz00_xz0x_1z11_00x1_01x1_1x0x_0zz0_x10z, 128'b00x0_xxx0_0xx1_00xx_x00x_xx1x_0x0x_x0x0_x000_x000_xx1x_x0x0_00x0_1xx0_x000_xx0x_0xxx_x00x_0x0x_0x1x_000x_001x_10x0_00x0_xx00_xx0x_xxx1_0000_0xx1_xx0x_00x0_x000);
		$vogls_assert_eq(128'b0zx0_zz11_1z00_zx01_zzz0_11zx_xx11_0x0x_01xx_zzzx_1z1z_xz0x_x0z0_1zz0_xz01_111x_x01z_z101_1111_10xx_010x_1011_zxzz_010x_1xzz_x010_01zx_1x11_1zzx_1zx0_10x1_zxz0 & 128'b1xxx_0z11_z1z0_0xzz_0zx1_x1zx_zzxx_xxxx_x000_00z0_xzx0_z10z_0zzz_1xxz_xx0z_zxxz_x1x1_zz1x_x01x_x0xz_x011_zzzx_01xx_0zx0_0z0z_1x0x_z10x_z1xz_zx0z_0xxz_11z0_zzzx, 128'b0xx0_0x11_xx00_0x0x_0xx0_x1xx_xxxx_0x0x_0000_00x0_xxx0_xx0x_00x0_1xx0_xx0x_xxxx_x0xx_xx0x_x01x_x0xx_000x_x0xx_0xxx_0x00_0x0x_x000_010x_xxxx_xx0x_0xx0_10x0_xxx0);

		$vogls_assert_eq(128'b1xz0_111x_x0x0_z0xx_xxx1_z1zz_xzxx_10zx_zxxz_z11z_0x0x_zz00_zzz0_z1zx_x1z1_z0z0_10xx_xz0x_1z1z_x0zz_010z_x0xx_zz00_x00x_0010_1xz1_z1x1_x1z0_x100_0x10_xz1z_x0z0 | 128'b111x_11zx_01x1_z1z1_zx0z_zz1z_1011_x0xx_0xzx_x0xx_xx0z_11xx_zz11_0xx0_x00z_0z01_zz01_z1z1_0xzz_110z_zx10_xxxx_zzx1_1zz0_xz0z_0zx1_x110_z111_z0z0_1z1x_0011_z1zx, 128'b111x_111x_x1x1_x1x1_xxx1_x11x_1x11_10xx_xxxx_x11x_xx0x_11xx_xx11_x1xx_x1x1_xxx1_1xx1_x1x1_1x1x_11xx_x11x_xxxx_xxx1_1xxx_xx1x_1xx1_x111_x111_x1x0_1x1x_xx11_x1xx);
		$vogls_assert_eq(128'b1zz1_z1x1_0100_zxzx_0x0x_z010_zzz0_1011_z0z1_x0x1_xxxz_zzx0_z1z1_zz1z_zz01_xz1x_zz11_z0x1_1z1x_0zz0_100z_zxx0_xx00_z0zz_1110_z0zx_x00z_01zx_zx11_10x0_1110_01zz | 128'bz00z_xz10_xxz1_zxzx_z101_zx00_x0zx_xzxz_xx0x_111x_0x0x_z00x_zzx0_z1x1_1zxz_0000_zz00_xxzz_zzx1_xxx1_x01z_01x1_x0xx_z1xz_00x1_1zx0_01zx_z11z_z001_zz1x_z1x0_00xx, 128'b1xx1_x111_x1x1_xxxx_x101_xx10_xxxx_1x11_xxx1_1111_xxxx_xxxx_x1x1_x111_1xx1_xx1x_xx11_xxx1_1x11_xxx1_101x_x1x1_xxxx_x1xx_1111_1xxx_x1xx_x11x_xx11_1x1x_1110_01xx);

		$vogls_assert_eq(128'b00x1_10x1_x1xz_0zx0_110z_01x1_0zz1_z0xz_x001_xxzx_x1zz_0z0x_zzzz_xxzx_z10z_zzxx_1xx1_01zx_zz0x_xxz0_0x10_zzz1_0101_zxzx_0z0x_x1zx_01zz_x1x0_z0zz_z011_0xxz_z1z0 ^ 128'b11xz_z110_xx00_x0xz_zxxx_z0x0_xxzx_1xzz_1x0x_1z11_zz0x_1xzx_x0xz_x0z0_xxz0_x001_xx10_1zz0_zx1x_01z0_0x0x_x0xx_10x1_xx0z_x1x1_1xzx_0x10_z0x0_0x00_00xx_xxzz_100x, 128'b11xx_x1x1_xxxx_xxxx_xxxx_x1x1_xxxx_xxxx_xx0x_xxxx_xxxx_1xxx_xxxx_xxxx_xxxx_xxxx_xxx1_1xxx_xx1x_xxx0_0x1x_xxxx_11x0_xxxx_xxxx_xxxx_0xxx_x1x0_xxxx_x0xx_xxxx_x1xx);
		$vogls_assert_eq(128'b1x0z_0101_x0z1_10x1_1zzx_zx10_xzx1_xzz0_010z_010x_0x01_x100_zx0z_x0z0_1xx1_011x_1zz1_zzxx_zzx0_xxzz_0x11_1xxx_z110_z111_00z0_0zx0_11z0_zxxz_110x_0x0x_xxz0_x1z0 ^ 128'b1x1z_xx1x_zxxx_0001_x1x0_0x1z_xz1z_000z_10x1_x11x_10zx_1z0x_01z1_xx00_110x_0x01_0x00_xzz0_x001_zzzx_z011_xz00_xzx1_xxzx_zz0x_x011_1001_01xz_0xzx_011z_10x0_1011, 128'b0x1x_xx1x_xxxx_10x0_xxxx_xx0x_xxxx_xxxx_11xx_x01x_1xxx_xx0x_xxxx_xxx0_0xxx_0x1x_1xx1_xxxx_xxx1_xxxx_xx00_xxxx_xxx1_xxxx_xxxx_xxx1_01x1_xxxx_1xxx_0x1x_xxx0_x1x1);
`endif

		// 129-bit
		$vogls_assert_eq(129'h00_0000_0000_0000_0000_0000_0000_0000_0000 & 129'h00_0000_0000_0000_0000_0000_0000_0000_0000, 129'h00_0000_0000_0000_0000_0000_0000_0000_0000);
		$vogls_assert_eq(129'h00_0000_0000_0000_0000_0000_0000_0000_0000 & 129'h01_ffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff, 129'h00_0000_0000_0000_0000_0000_0000_0000_0000);
		$vogls_assert_eq(129'h01_ffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff & 129'h00_0000_0000_0000_0000_0000_0000_0000_0000, 129'h00_0000_0000_0000_0000_0000_0000_0000_0000);
		$vogls_assert_eq(129'h01_ffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff & 129'h01_ffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff, 129'h01_ffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff);
		$vogls_assert_eq(129'h00_5631_3ee4_84e9_b1a8_5e63_64e7_ec96_3a65 & 129'h01_9d0b_6e9f_1595_2fef_093b_0e91_eb17_2261, 129'h00_1401_2e84_0481_21a8_0823_0481_e816_2261);
		$vogls_assert_eq(129'h01_ee78_b485_aa9c_5abd_a6ce_14bb_b52e_69d4 & 129'h00_f94b_1660_4d32_a77f_59ce_ebab_c69f_78b2, 129'h00_e848_1400_0810_023d_00ce_00ab_840e_6890);
		$vogls_assert_eq(129'h00_5542_3ea3_d71f_5d55_68e7_4dc8_951c_fe3e & 129'h00_6963_28bb_6ac3_a28a_946b_653f_cd6c_ee3e, 129'h00_4142_28a3_4203_0000_0063_4508_850c_ee3e);
		$vogls_assert_eq(129'h01_c132_3f4e_0b36_482f_f7fb_1154_32bd_8963 & 129'h00_800a_f688_5aca_3400_f9b3_2551_a7f9_3bc3, 129'h00_8002_3608_0a02_0000_f1b3_0150_22b9_0943);

		$vogls_assert_eq(129'h00_0000_0000_0000_0000_0000_0000_0000_0000 | 129'h00_0000_0000_0000_0000_0000_0000_0000_0000, 129'h00_0000_0000_0000_0000_0000_0000_0000_0000);
		$vogls_assert_eq(129'h00_0000_0000_0000_0000_0000_0000_0000_0000 | 129'h01_ffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff, 129'h01_ffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff);
		$vogls_assert_eq(129'h01_ffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff | 129'h00_0000_0000_0000_0000_0000_0000_0000_0000, 129'h01_ffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff);
		$vogls_assert_eq(129'h01_ffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff | 129'h01_ffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff, 129'h01_ffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff);
		$vogls_assert_eq(129'h01_5a61_c59d_b057_95a4_a50a_2966_ae1c_6a20 | 129'h01_76a4_3c4d_05c5_9100_2ece_d166_e3ac_d340, 129'h01_7ee5_fddd_b5d7_95a4_afce_f966_efbc_fb60);
		$vogls_assert_eq(129'h01_6a67_60e0_5b43_2c5e_9d57_ed85_26fd_8fee | 129'h00_1ce2_7fd2_15aa_8752_feeb_1a0d_9a66_5775, 129'h01_7ee7_7ff2_5feb_af5e_ffff_ff8d_beff_dfff);
		$vogls_assert_eq(129'h01_bd41_e3e3_2a1f_ab18_fc30_f2cf_5fed_a332 | 129'h00_26ee_09d2_a0c5_a5ba_e7ac_b2c1_c1a7_821e, 129'h01_bfef_ebf3_aadf_afba_ffbc_f2cf_dfef_a33e);
		$vogls_assert_eq(129'h00_3d23_940b_0496_f72a_c55a_9e34_97cc_2393 | 129'h00_e591_76ac_9e73_5bc9_d4a0_a602_b9f2_1aa7, 129'h00_fdb3_f6af_9ef7_ffeb_d5fa_be36_bffe_3bb7);

		$vogls_assert_eq(129'h00_0000_0000_0000_0000_0000_0000_0000_0000 ^ 129'h00_0000_0000_0000_0000_0000_0000_0000_0000, 129'h00_0000_0000_0000_0000_0000_0000_0000_0000);
		$vogls_assert_eq(129'h00_0000_0000_0000_0000_0000_0000_0000_0000 ^ 129'h01_ffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff, 129'h01_ffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff);
		$vogls_assert_eq(129'h01_ffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff ^ 129'h00_0000_0000_0000_0000_0000_0000_0000_0000, 129'h01_ffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff);
		$vogls_assert_eq(129'h01_ffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff ^ 129'h01_ffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff, 129'h00_0000_0000_0000_0000_0000_0000_0000_0000);
		$vogls_assert_eq(129'h01_db0b_2406_029f_5fb2_b278_9403_8c68_f71b ^ 129'h01_a9b8_0566_417f_f49e_8380_9aa9_806e_c74f, 129'h00_72b3_2160_43e0_ab2c_31f8_0eaa_0c06_3054);
		$vogls_assert_eq(129'h00_6fb4_d48c_7057_c650_1f08_4e94_cf32_280f ^ 129'h01_5c5d_c12f_b574_a57e_3e70_3415_ac6f_7b79, 129'h01_33e9_15a3_c523_632e_2178_7a81_635d_5376);
		$vogls_assert_eq(129'h01_134e_31da_550b_3c19_a8a8_67df_9457_24f9 ^ 129'h01_c3da_6dfd_887a_e7b9_e3f6_abf7_8956_b3c5, 129'h00_d094_5c27_dd71_dba0_4b5e_cc28_1d01_973c);
		$vogls_assert_eq(129'h01_dfb8_1b7f_9fa7_391f_e5ea_51e7_5c41_4647 ^ 129'h00_e318_82f4_4d94_021e_c39d_24b2_a2e1_8d4a, 129'h01_3ca0_998b_d233_3b01_2677_7555_fea0_cb0d);

`ifndef __VOGLS__TWO_VALUE_LOGIC
		$vogls_assert_eq(129'bx_1z10_00zz_00zz_1zzx_x0zx_z01x_1xx1_xz10_x00x_xz0x_z01x_1x00_z000_z01z_001z_zxxx_0zx0_1011_xx0x_x0z1_0111_1zxz_z001_0000_zz0z_x1xx_1xzz_01zz_10xz_z1x1_0zzx_0x01 & 129'bx_101x_zx0x_10xz_1xzz_xz01_zx00_x0x0_zx10_1xx0_0z0z_zx01_zxz1_xzx0_x0zz_10z0_1xxz_zzzx_10z0_x11x_zx1x_0xx1_xzzx_0xx0_1zxx_0xzz_1z0z_1110_x1x0_x11z_1x1z_xz10_10x0, 129'bx_1010_000x_00xx_1xxx_x00x_x000_x0x0_xx10_x000_0x0x_x00x_xx00_x000_x0xx_00x0_xxxx_0xx0_10x0_xx0x_x0xx_0xx1_xxxx_0000_0000_0x0x_xx0x_1xx0_01x0_x0xx_xxxx_0xx0_0000);
		$vogls_assert_eq(129'b0_z01x_1z1z_0x1z_x0z1_zz0x_011z_z1x0_xzxx_0x1x_1000_1011_0zz1_xz10_x1x1_x0xz_zxxx_x010_z00x_z0xz_1z1z_0111_0zxx_z0x0_zzxx_0xz1_zx1x_x01z_zzz1_0xz0_011x_00x0_0zz0 & 129'bx_0x11_0xz0_x0zz_1011_0x1z_x0x0_z100_zxxz_1zzx_zz1z_10xx_01xx_x0zz_0x1z_11x1_zx0x_xx0x_zxxz_00z1_x101_x1x1_1z01_110x_zx1z_0zzz_0zxx_xzx0_000x_xz11_xz01_0xzz_0zz0, 129'b0_001x_0xx0_00xx_x0x1_0x0x_00x0_x100_xxxx_0xxx_x000_10xx_0xxx_x0x0_0xxx_x0xx_xx0x_x000_x00x_00xx_xx0x_01x1_0x0x_x000_xxxx_0xxx_0xxx_x0x0_000x_0xx0_0x0x_00x0_0xx0);

		$vogls_assert_eq(129'bz_100z_z0xx_101z_1x10_11x1_1x1x_zz01_01x1_01z0_xzzx_x111_1x1z_1010_11xz_1xzz_zxxz_zxx0_0zx1_11z1_01zx_x10x_000z_01z1_zzzx_z00z_110z_1011_xxz1_x0zx_x00x_001z_z011 | 129'b0_zx0z_xxzz_00x0_zz10_xzxz_11z1_01xz_110x_0z1z_0xxx_x001_0010_1zxz_101x_x1xz_110z_zx0z_z01z_11zx_x000_xz1x_1z1x_0x10_01z0_1010_xz0x_10z0_00xz_z10z_xxx1_00z0_10zz, 129'bx_1x0x_xxxx_101x_1x10_11x1_1111_x1x1_11x1_011x_xxxx_x111_1x1x_1x1x_111x_11xx_11xx_xxxx_xx11_11x1_x1xx_x11x_1x1x_0111_x1xx_101x_110x_1011_xxx1_x1xx_xxx1_001x_1011);
		$vogls_assert_eq(129'bx_1z01_1110_1xzx_zxxx_1zx0_xzz1_01zz_11zx_01zx_11z0_zx1x_00zz_1zx1_xzz1_x100_z0x0_1z0z_1x00_z1z1_z0x0_z0z1_x11z_x0z1_z10x_x1x0_0xzx_1z0x_z1zx_xx0x_xz0x_0001_z0x1 | 129'b0_1x0z_xxxx_01xx_0x01_xxzz_1xz0_1x1z_zx01_1zz1_0x01_x011_10z0_x01x_xxz1_1z0x_xzx0_0011_x011_0000_xzxz_0010_z11x_1z1x_z1z1_01zz_1111_zx11_z00x_0zzx_xxz0_011x_0xxz, 129'bx_1x01_111x_11xx_xxx1_1xxx_1xx1_111x_11x1_11x1_11x1_xx11_10xx_1x11_xxx1_110x_xxx0_1x11_1x11_x1x1_xxxx_x011_x11x_1x11_x1x1_x1xx_1111_1x11_x1xx_xxxx_xxxx_0111_xxx1);

		$vogls_assert_eq(129'bx_01z0_111z_0100_0x0z_0000_10zx_1z10_1xxx_z11x_xx0z_0z00_11xz_z0x0_010z_zxz1_11zz_z1xx_x0zx_xz1z_01z1_x1z1_0zx1_zx01_0zz1_0z01_000z_0x1z_x000_xzxz_xx0z_z011_zx01 ^ 129'b1_1xxz_1x0z_xxz1_1zx1_z1x0_xxx1_0z01_0z11_xx00_1z11_1zx0_x011_01zx_1z01_0xz0_1110_xz00_x1zz_1z00_z1xz_z100_z0xx_1z0x_1zz1_00x0_zz01_zx11_000z_110z_xx10_xx1x_0xzz, 129'bx_1xxx_0x1x_xxx1_1xxx_x1x0_xxxx_1x11_1xxx_xx1x_xx1x_1xx0_x1xx_x1xx_1x0x_xxx1_00xx_xxxx_x1xx_xx1x_x0xx_x0x1_xxxx_xx0x_1xx0_0xx1_xx0x_xx0x_x00x_xxxx_xx1x_xx0x_xxxx);
		$vogls_assert_eq(129'bz_1x1x_01x1_0001_11xx_x0zz_z01z_0010_x10z_1zxx_z111_z110_zzz0_x1xx_x00x_1z10_0z10_z000_10zx_00xx_xz00_000x_1x10_0110_xx1x_z000_x1x1_zxx1_z10z_10z0_z11x_zz11_1xxz ^ 129'bz_xzx0_10x0_zz0x_xx01_0xx1_xx11_xzz0_0zz1_01xx_1zx1_011z_x101_10z0_xx1x_010z_z0xz_xz10_z110_zzz0_z0zz_x101_1zzx_0x00_zz0z_zx1x_zzx1_001z_z00z_xz1x_011x_0x00_0z1z, 129'bx_xxxx_11x1_xx0x_xxxx_xxxx_xx0x_xxx0_xxxx_1xxx_xxx0_x00x_xxx1_x1xx_xx1x_1x1x_xxxx_xx10_x1xx_xxxx_xxxx_x10x_0xxx_0x10_xx1x_xx1x_xxx0_xxxx_x10x_xxxx_x00x_xx11_1xxx);
`endif
    end
endmodule
