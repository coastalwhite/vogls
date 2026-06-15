module x();
	initial begin
		$vogls_assert_eq(32'hABCD_EF01 <<  0, 32'hABCD_EF01);
		$vogls_assert_eq(32'hABCD_EF01 <<  1, 32'h579B_DE02);
		$vogls_assert_eq(32'hABCD_EF01 <<  2, 32'hAF37_BC04);
		$vogls_assert_eq(32'hABCD_EF01 <<  8, 32'hCDEF_0100);
		$vogls_assert_eq(32'hABCD_EF01 << 11, 32'h6F78_0800);

		$vogls_assert_eq(32'hABCD_EF01 >>  0, 32'hABCD_EF01);
		$vogls_assert_eq(32'hABCD_EF01 >>  1, 32'h55E6_F780);
		$vogls_assert_eq(32'hABCD_EF01 >>  2, 32'h2AF3_7BC0);
		$vogls_assert_eq(32'hABCD_EF01 >>  8, 32'h00AB_CDEF);
		$vogls_assert_eq(32'hABCD_EF01 >> 11, 32'h0015_79BD);


		`define Y 128'habcd_ef01_1372_1921_abc1_2319_4329_2342
		$vogls_assert_eq(`Y <<  0, `Y);
		$vogls_assert_eq(`Y <<  1, 128'h579b_de02_26e4_3243_5782_4632_8652_4684);
		$vogls_assert_eq(`Y <<  2, 128'haf37_bc04_4dc8_6486_af04_8c65_0ca4_8d08);
		$vogls_assert_eq(`Y <<  3, 128'h5e6f_7808_9b90_c90d_5e09_18ca_1949_1a10);
		$vogls_assert_eq(`Y <<  7, 128'he6f7_8089_b90c_90d5_e091_8ca1_9491_a100);
		$vogls_assert_eq(`Y <<  8, 128'hcdef_0113_7219_21ab_c123_1943_2923_4200);
		$vogls_assert_eq(`Y << 11, 128'h6f78_089b_90c9_0d5e_0918_ca19_491a_1000);
		$vogls_assert_eq(`Y << 35, 128'h9b90_c90d_5e09_18ca_1949_1a10_0000_0000);

		$vogls_assert_eq(`Y >>  0, `Y);
		$vogls_assert_eq(`Y >>  1, 128'h55e6_f780_89b9_0c90_d5e0_918c_a194_91a1);
		$vogls_assert_eq(`Y >>  2, 128'h2af3_7bc0_44dc_8648_6af0_48c6_50ca_48d0);
		$vogls_assert_eq(`Y >>  3, 128'h1579_bde0_226e_4324_3578_2463_2865_2468);
		$vogls_assert_eq(`Y >>  7, 128'h0157_9bde_0226_e432_4357_8246_3286_5246);
		$vogls_assert_eq(`Y >>  8, 128'h00ab_cdef_0113_7219_21ab_c123_1943_2923);
		$vogls_assert_eq(`Y >> 11, 128'h0015_79bd_e022_6e43_2435_7824_6328_6524);
		$vogls_assert_eq(`Y >> 35, 128'h0000_0000_1579_bde0_226e_4324_3578_2463);

		`define Z 97'h1_1372_1921_abc1_2319_4329_2342
		$vogls_assert_eq(`Z <<  0, `Z);
		$vogls_assert_eq(`Z <<  1, 97'h0_26e4_3243_5782_4632_8652_4684);
		$vogls_assert_eq(`Z <<  2, 97'h0_4dc8_6486_af04_8c65_0ca4_8d08);
		$vogls_assert_eq(`Z <<  3, 97'h0_9b90_c90d_5e09_18ca_1949_1a10);
		$vogls_assert_eq(`Z <<  7, 97'h1_b90c_90d5_e091_8ca1_9491_a100);
		$vogls_assert_eq(`Z <<  8, 97'h1_7219_21ab_c123_1943_2923_4200);
		$vogls_assert_eq(`Z << 11, 97'h1_90c9_0d5e_0918_ca19_491a_1000);
		$vogls_assert_eq(`Z << 35, 97'h1_5e09_18ca_1949_1a10_0000_0000);

		$vogls_assert_eq(`Z >>  0, `Z);
		$vogls_assert_eq(`Z >>  1, 97'h0_89b9_0c90_d5e0_918c_a194_91a1);
		$vogls_assert_eq(`Z >>  2, 97'h0_44dc_8648_6af0_48c6_50ca_48d0);
		$vogls_assert_eq(`Z >>  3, 97'h0_226e_4324_3578_2463_2865_2468);
		$vogls_assert_eq(`Z >>  7, 97'h0_0226_e432_4357_8246_3286_5246);
		$vogls_assert_eq(`Z >>  8, 97'h0_0113_7219_21ab_c123_1943_2923);
		$vogls_assert_eq(`Z >> 11, 97'h0_0022_6e43_2435_7824_6328_6524);
		$vogls_assert_eq(`Z >> 35, 97'h0_0000_0000_226e_4324_3578_2463);
	end
endmodule
