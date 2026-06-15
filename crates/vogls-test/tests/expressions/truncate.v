module tb();
    reg a;
    reg[7:0] b;
    reg[31:0] c;
    reg[32:0] d;
    reg[127:0] e;
    reg[128:0] f;

    initial begin
        // a = 2'b11; $vogls_assert_eq(a, 1'b1);
        // a = 2'b01; $vogls_assert_eq(a, 1'b1);
        // a = 2'b10; $vogls_assert_eq(a, 1'b0);
`ifndef __VOGLS__TWO_VALUE_LOGIC
        a = 2'bx0; $vogls_assert_eq(a, 1'b0);
        a = 2'bz0; $vogls_assert_eq(a, 1'b0);
        a = 2'b0x; $vogls_assert_eq(a, 1'bx);
        // a = 2'b0z; $vogls_assert_eq(a, 1'bz);
`endif
/*

        a = 17'h0_0000; $vogls_assert_eq(a, 1'b0);
        a = 17'h0_0001; $vogls_assert_eq(a, 1'b1);
        a = 17'h1_0000; $vogls_assert_eq(a, 1'b0);
`ifndef __VOGLS__TWO_VALUE_LOGIC
        a = 17'h0_x000; $vogls_assert_eq(a, 1'b0);
        a = 17'h0_z000; $vogls_assert_eq(a, 1'b0);
        a = 17'h0_100x; $vogls_assert_eq(a, 1'bx);
        a = 17'h0_100z; $vogls_assert_eq(a, 1'bz);
`endif

        a = 128'h00000000_00000000_00000000_00000000; $vogls_assert_eq(a, 1'b0);
        a = 128'h00000000_00000000_00000000_00000001; $vogls_assert_eq(a, 1'b1);
        a = 128'hFFFFFFFF_00000000_00000000_00000000; $vogls_assert_eq(a, 1'b0);
        a = 129'h0_00000000_00000000_00000000_00000001; $vogls_assert_eq(a, 1'b1);
`ifndef __VOGLS__TWO_VALUE_LOGIC
        a = 128'h00xxx000_00000000_00000000_00000000; $vogls_assert_eq(a, 1'b0);
        a = 128'h00zzz000_00000000_00000000_00000001; $vogls_assert_eq(a, 1'b1);
        a = 128'h00000000_00000000_00000000_0000000x; $vogls_assert_eq(a, 1'bx);
        a = 128'hFFFFFFFF_00000000_00000000_0000000z; $vogls_assert_eq(a, 1'bz);
        a = 129'h1_FFFFFFFF_00000000_00000000_0000000z; $vogls_assert_eq(a, 1'bz);
`endif

        b = 32'h594c_7059; $vogls_assert_eq(b, 8'h59);
        b = 33'h0_594c_7059; $vogls_assert_eq(b, 8'h59);
        b = 33'h1_62be_ecb8; $vogls_assert_eq(b, 8'hb8);
        b = 33'h0_aff2_ce29; $vogls_assert_eq(b, 8'h29);
`ifndef __VOGLS__TWO_VALUE_LOGIC
        b = 33'h0_bex5_c627; $vogls_assert_eq(b, 8'h27);
        b = 33'hz_a915_3cc2; $vogls_assert_eq(b, 8'hc2);
        b = 33'h1_48d0_xxxx; $vogls_assert_eq(b, 8'hxx);
        b = 33'hx_c1ca_09b5; $vogls_assert_eq(b, 8'hb5);
`endif

        b = 128'h0366_1b77_957f_2f9c_f800_0cac_bc37_ff87; $vogls_assert_eq(b, 8'h87);
        b = 128'hf9d1_bb42_6489_b23f_07e0_bfb9_5ea3_1004; $vogls_assert_eq(b, 8'h04);
        b = 128'habbc_ec5f_f68f_bbf9_82d0_f9df_67f7_c682; $vogls_assert_eq(b, 8'h82);
        b = 129'h1_abbc_ec5f_f68f_bbf9_82d0_f9df_67f7_c682; $vogls_assert_eq(b, 8'h82);
`ifndef __VOGLS__TWO_VALUE_LOGIC
        b = 128'h3xxx_7fbd_ce59_523b_55a8_ea72_f81f_6235; $vogls_assert_eq(b, 8'h35);
        b = 128'h542d_6zzz_7d0f_ae2b_dc53_0742_cc44_8c8a; $vogls_assert_eq(b, 8'h8a);
        b = 128'h259e_d32c_3ed7_c969_f276_21a2_fxxx_880f; $vogls_assert_eq(b, 8'h0f);
        b = 128'h673c_05a5_8b41_860a_4fcb_93ad_9a0c_zzzx; $vogls_assert_eq(b, 8'hzx);
        b = 129'h0_673c_05a5_8b41_860a_4fcb_93ad_9a0c_zzzx; $vogls_assert_eq(b, 8'hzx);
`endif

        c = 33'h0_594c_7059; $vogls_assert_eq(c, 32'h594c_7059);
        c = 33'h1_62be_ecb8; $vogls_assert_eq(c, 32'h62be_ecb8);
        c = 33'h0_aff2_ce29; $vogls_assert_eq(c, 32'haff2_ce29);
`ifndef __VOGLS__TWO_VALUE_LOGIC
        c = 33'h0_bex5_c627; $vogls_assert_eq(c, 32'hbex5_c627);
        c = 33'hz_a915_3cc2; $vogls_assert_eq(c, 32'ha915_3cc2);
        c = 33'h1_48d0_xxxx; $vogls_assert_eq(c, 32'h48d0_xxxx);
        c = 33'hx_c1ca_09b5; $vogls_assert_eq(c, 32'hc1ca_09b5);
`endif

        c = 128'h0366_1b77_957f_2f9c_f800_0cac_bc37_ff87; $vogls_assert_eq(c, 32'hbc37_ff87);
        c = 128'hf9d1_bb42_6489_b23f_07e0_bfb9_5ea3_1004; $vogls_assert_eq(c, 32'h5ea3_1004);
        c = 128'habbc_ec5f_f68f_bbf9_82d0_f9df_67f7_c682; $vogls_assert_eq(c, 32'h67f7_c682);
        c = 129'h1_abbc_ec5f_f68f_bbf9_82d0_f9df_67f7_c682; $vogls_assert_eq(c, 32'h67f7_c682);
`ifndef __VOGLS__TWO_VALUE_LOGIC
        c = 128'h3xxx_7fbd_ce59_523b_55a8_ea72_f81f_6235; $vogls_assert_eq(c, 32'hf81f_6235);
        c = 128'h542d_6zzz_7d0f_ae2b_dc53_0742_cc44_8c8a; $vogls_assert_eq(c, 32'hcc44_8c8a);
        c = 128'h259e_d32c_3ed7_c969_f276_21a2_fxxx_880f; $vogls_assert_eq(c, 32'hfxxx_880f);
        c = 128'h673c_05a5_8b41_860a_4fcb_93ad_9a0c_zzzx; $vogls_assert_eq(c, 32'h9a0c_zzzx);
        c = 129'h1_673c_05a5_8b41_860a_4fcb_93ad_9a0c_zzzx; $vogls_assert_eq(c, 32'h9a0c_zzzx);
`endif

        d = 35'h05_43d8_ba4d; $vogls_assert_eq(d, 33'h1_43d8_ba4d);
        d = 35'h03_ae07_d9a4; $vogls_assert_eq(d, 33'h1_ae07_d9a4);
        d = 35'h05_a9b3_e2c1; $vogls_assert_eq(d, 33'h1_a9b3_e2c1);
        d = 64'hffda_d668_32bd_24f8; $vogls_assert_eq(d, 33'h0_32bd_24f8);

`ifndef __VOGLS__TWO_VALUE_LOGIC
        d = 35'hx1_f2c5_6b93; $vogls_assert_eq(d, 33'h1_f2c5_6b93);
        d = 35'hz6_ff3c_1d24; $vogls_assert_eq(d, 33'h0_ff3c_1d24);
        d = 35'h01_5xxe_3776; $vogls_assert_eq(d, 33'h1_5xxe_3776);
        d = 35'h03_b7c6_2xzx; $vogls_assert_eq(d, 33'h1_b7c6_2xzx);
        d = 64'hffda_d668_32bd_2xz8; $vogls_assert_eq(d, 33'h0_32bd_2xz8);
`endif

        d = 128'h310c_5c2b_c053_9901_5df9_ad89_8e71_aae9; $vogls_assert_eq(d, 33'h1_8e71_aae9);
        d = 128'h54e7_7697_5b80_9c8a_5eab_a555_cea3_e959; $vogls_assert_eq(d, 33'h1_cea3_e959);
        d = 128'h7764_47bf_60af_48c8_5fa2_0c5b_603f_d410; $vogls_assert_eq(d, 33'h1_603f_d410);
        d = 128'h7f84_363c_ca2f_da18_dd93_4f9f_8a07_497d; $vogls_assert_eq(d, 33'h1_8a07_497d);
        d = 129'h1_7f84_363c_ca2f_da18_dd93_4f9f_8a07_497d; $vogls_assert_eq(d, 33'h1_8a07_497d);
`ifndef __VOGLS__TWO_VALUE_LOGIC
        d = 128'h37zd_3136_fxze_2zz0_19f2_xz59_ef1b_cxdc; $vogls_assert_eq(d, 33'h1_ef1b_cxdc);
        d = 128'h1xxe_c6ac_0b11_b6b7_9770_b2c0_edb2_f7b0; $vogls_assert_eq(d, 33'h0_edb2_f7b0);
        d = 128'h533f_93a2_64b2_945a_8cd1_d09e_2xzx_7a08; $vogls_assert_eq(d, 33'h0_2xzx_7a08);
        d = 128'h4996_7157_153b_4946_f1d4_1fcx_8a29_9a69; $vogls_assert_eq(d, 33'hx_8a29_9a69);
        d = 129'h1_4996_7157_153b_4946_f1d4_1fcx_8a29_9a69; $vogls_assert_eq(d, 33'hx_8a29_9a69);
`endif

        e = 256'h2bfb_de62_178f_8450_23b9_e430_0f82_c278_a472_9466_baf0_b1c7_755a_be87_2af7_9a8c; $vogls_assert_eq(e, 128'ha472_9466_baf0_b1c7_755a_be87_2af7_9a8c);
        e = 256'hd4af_1cd7_dab2_0c25_b63a_15d1_1c7c_1b11_619f_1a19_ad00_8990_2a17_80a8_0a76_a35e; $vogls_assert_eq(e, 128'h619f_1a19_ad00_8990_2a17_80a8_0a76_a35e);
        e = 256'h2878_55ea_3487_39ff_c2c3_46c7_214a_87ac_66d6_e959_ca97_06fc_065a_2bec_3fa8_c982; $vogls_assert_eq(e, 128'h66d6_e959_ca97_06fc_065a_2bec_3fa8_c982);
        e = 256'h57b2_4d8c_d03f_f6f1_c5b3_5ae2_b317_8199_7079_2502_606c_37f4_34dd_a0ed_db05_1234; $vogls_assert_eq(e, 128'h7079_2502_606c_37f4_34dd_a0ed_db05_1234);
        e = 211'h03_58b7_a8fa_68f8_4972_9e34_84ac_7358_ffdf_22ff_4b65_d44b_510d_f10c; $vogls_assert_eq(e, 128'h84ac_7358_ffdf_22ff_4b65_d44b_510d_f10c);
        e = 211'h05_2780_caed_ade8_b2c1_1515_90e3_2005_8a66_022b_433f_2c58_922d_b2cd; $vogls_assert_eq(e, 128'h90e3_2005_8a66_022b_433f_2c58_922d_b2cd);
        e = 211'h00_3b22_d66b_95bc_abd3_38b5_696d_9a8b_c5ea_f247_a118_63ec_6aa6_cd53; $vogls_assert_eq(e, 128'h696d_9a8b_c5ea_f247_a118_63ec_6aa6_cd53);
        e = 211'h04_a8f5_12fa_12ea_f962_8cd7_d11a_7c69_5c0f_3ee1_d9a3_f0c5_52c1_eca2; $vogls_assert_eq(e, 128'hd11a_7c69_5c0f_3ee1_d9a3_f0c5_52c1_eca2);
`ifndef __VOGLS__TWO_VALUE_LOGIC
        e = 211'h03_58b7_a8fa_68f8_4972_9e34_84ac_7358_ffdf_22ff_4b65_d44b_510d_xxzc; $vogls_assert_eq(e, 128'h84ac_7358_ffdf_22ff_4b65_d44b_510d_xxzc);
        e = 211'h05_2780_caed_ade8_xxz1_1515_90e3_2005_8a66_022b_433f_2c58_922d_b2cd; $vogls_assert_eq(e, 128'h90e3_2005_8a66_022b_433f_2c58_922d_b2cd);
        e = 211'h00_3b22_d66b_95bc_abd3_38b5_xzxx_9a8b_c5ea_f247_a118_63ec_6aa6_cd53; $vogls_assert_eq(e, 128'hxzxx_9a8b_c5ea_f247_a118_63ec_6aa6_cd53);
        e = 211'h0x_a8f5_12fa_12ea_f962_8cd7_d11a_7c69_5c0f_3ee1_d9a3_f0c5_52c1_eczz; $vogls_assert_eq(e, 128'hd11a_7c69_5c0f_3ee1_d9a3_f0c5_52c1_eczz);
`endif

        f = 256'h2bfb_de62_178f_8450_23b9_e430_0f82_c278_a472_9466_baf0_b1c7_755a_be87_2af7_9a8c; $vogls_assert_eq(f, 129'h0_a472_9466_baf0_b1c7_755a_be87_2af7_9a8c);
        f = 256'hd4af_1cd7_dab2_0c25_b63a_15d1_1c7c_1b11_619f_1a19_ad00_8990_2a17_80a8_0a76_a35e; $vogls_assert_eq(f, 129'h1_619f_1a19_ad00_8990_2a17_80a8_0a76_a35e);
        f = 256'h2878_55ea_3487_39ff_c2c3_46c7_214a_87ac_66d6_e959_ca97_06fc_065a_2bec_3fa8_c982; $vogls_assert_eq(f, 129'h0_66d6_e959_ca97_06fc_065a_2bec_3fa8_c982);
        f = 256'h57b2_4d8c_d03f_f6f1_c5b3_5ae2_b317_8199_7079_2502_606c_37f4_34dd_a0ed_db05_1234; $vogls_assert_eq(f, 129'h1_7079_2502_606c_37f4_34dd_a0ed_db05_1234);
        f = 211'h03_58b7_a8fa_68f8_4972_9e34_84ac_7358_ffdf_22ff_4b65_d44b_510d_f10c; $vogls_assert_eq(f, 129'h0_84ac_7358_ffdf_22ff_4b65_d44b_510d_f10c);
        f = 211'h05_2780_caed_ade8_b2c1_1515_90e3_2005_8a66_022b_433f_2c58_922d_b2cd; $vogls_assert_eq(f, 129'h1_90e3_2005_8a66_022b_433f_2c58_922d_b2cd);
        f = 211'h00_3b22_d66b_95bc_abd3_38b5_696d_9a8b_c5ea_f247_a118_63ec_6aa6_cd53; $vogls_assert_eq(f, 129'h1_696d_9a8b_c5ea_f247_a118_63ec_6aa6_cd53);
        f = 211'h04_a8f5_12fa_12ea_f962_8cd7_d11a_7c69_5c0f_3ee1_d9a3_f0c5_52c1_eca2; $vogls_assert_eq(f, 129'h1_d11a_7c69_5c0f_3ee1_d9a3_f0c5_52c1_eca2);
`ifndef __VOGLS__TWO_VALUE_LOGIC
        f = 211'h03_58b7_a8fa_68f8_4972_9e34_84ac_7358_ffdf_22ff_4b65_d44b_510d_xxzc; $vogls_assert_eq(f, 129'h0_84ac_7358_ffdf_22ff_4b65_d44b_510d_xxzc);
        f = 211'h05_2780_caed_ade8_xxz1_1515_90e3_2005_8a66_022b_433f_2c58_922d_b2cd; $vogls_assert_eq(f, 129'h1_90e3_2005_8a66_022b_433f_2c58_922d_b2cd);
        f = 211'h00_3b22_d66b_95bc_abd3_38b5_xzxx_9a8b_c5ea_f247_a118_63ec_6aa6_cd53; $vogls_assert_eq(f, 129'h1_xzxx_9a8b_c5ea_f247_a118_63ec_6aa6_cd53);
        f = 211'h0x_a8f5_12fa_12ea_f962_8cd7_d11a_7c69_5c0f_3ee1_d9a3_f0c5_52c1_eczz; $vogls_assert_eq(f, 129'h1_d11a_7c69_5c0f_3ee1_d9a3_f0c5_52c1_eczz);
`endif
*/
    end
endmodule
