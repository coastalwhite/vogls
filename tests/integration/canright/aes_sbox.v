`define A2X 64'hFF_A9_81_09_48_F2_F3_98
`define X2A 64'h60_DE_29_68_8C_6E_78_64
`define X2S 64'h24_03_04_DC_0B_9E_2D_58
`define S2X 64'h53_51_04_12_EB_05_79_8C

`define NO_TB
`include "./aes_mvn.v"
`include "./aes_inverse_gf2p8.v"

module sbox_fwd(data_i, data_o);
    input [7:0]  data_i;
    output [7:0] data_o;

    wire [7:0] data_basis_x;
    wire [7:0] data_inverse;
    wire [7:0] data_basis_s;

    assign data_o = data_basis_s ^ 8'h63;

    aes_mvn           a2x( .vec(data_i),          .mat(`A2X), .data_o(data_basis_x) );
    aes_inverse_gf2p8 inv( .data_i(data_basis_x),             .data_o(data_inverse) );
    aes_mvn           x2s( .vec(data_inverse),    .mat(`X2S), .data_o(data_basis_s) );
endmodule

module sbox_inv(data_i, data_o);
    input [7:0]  data_i;
    output [7:0] data_o;

    wire [7:0] data_basis_x;
    wire [7:0] data_inverse;

    aes_mvn           s2x( .vec   (data_i ^ 8'h63), .mat(`S2X), .data_o(data_basis_x) );
    aes_inverse_gf2p8 inv( .data_i(data_basis_x),               .data_o(data_inverse) );
    aes_mvn           x2a( .vec   (data_inverse),   .mat(`X2A), .data_o(data_o)       );
endmodule

module tb();
	wire [ 256 * 8 - 1 : 0] SBOX_LUT;
	wire [ 256 * 8 - 1 : 0] SBOX_INV_LUT;
	assign SBOX_LUT = {
      //  0     1     2     3     4     5     6     7     8     9     a     b     c     d     e     f
      8'h63,8'h7c,8'h77,8'h7b,8'hf2,8'h6b,8'h6f,8'hc5,8'h30,8'h01,8'h67,8'h2b,8'hfe,8'hd7,8'hab,8'h76, // 0
      8'hca,8'h82,8'hc9,8'h7d,8'hfa,8'h59,8'h47,8'hf0,8'had,8'hd4,8'ha2,8'haf,8'h9c,8'ha4,8'h72,8'hc0, // 1
      8'hb7,8'hfd,8'h93,8'h26,8'h36,8'h3f,8'hf7,8'hcc,8'h34,8'ha5,8'he5,8'hf1,8'h71,8'hd8,8'h31,8'h15, // 2
      8'h04,8'hc7,8'h23,8'hc3,8'h18,8'h96,8'h05,8'h9a,8'h07,8'h12,8'h80,8'he2,8'heb,8'h27,8'hb2,8'h75, // 3
      8'h09,8'h83,8'h2c,8'h1a,8'h1b,8'h6e,8'h5a,8'ha0,8'h52,8'h3b,8'hd6,8'hb3,8'h29,8'he3,8'h2f,8'h84, // 4
      8'h53,8'hd1,8'h00,8'hed,8'h20,8'hfc,8'hb1,8'h5b,8'h6a,8'hcb,8'hbe,8'h39,8'h4a,8'h4c,8'h58,8'hcf, // 5
      8'hd0,8'hef,8'haa,8'hfb,8'h43,8'h4d,8'h33,8'h85,8'h45,8'hf9,8'h02,8'h7f,8'h50,8'h3c,8'h9f,8'ha8, // 6
      8'h51,8'ha3,8'h40,8'h8f,8'h92,8'h9d,8'h38,8'hf5,8'hbc,8'hb6,8'hda,8'h21,8'h10,8'hff,8'hf3,8'hd2, // 7
      8'hcd,8'h0c,8'h13,8'hec,8'h5f,8'h97,8'h44,8'h17,8'hc4,8'ha7,8'h7e,8'h3d,8'h64,8'h5d,8'h19,8'h73, // 8
      8'h60,8'h81,8'h4f,8'hdc,8'h22,8'h2a,8'h90,8'h88,8'h46,8'hee,8'hb8,8'h14,8'hde,8'h5e,8'h0b,8'hdb, // 9
      8'he0,8'h32,8'h3a,8'h0a,8'h49,8'h06,8'h24,8'h5c,8'hc2,8'hd3,8'hac,8'h62,8'h91,8'h95,8'he4,8'h79, // a
      8'he7,8'hc8,8'h37,8'h6d,8'h8d,8'hd5,8'h4e,8'ha9,8'h6c,8'h56,8'hf4,8'hea,8'h65,8'h7a,8'hae,8'h08, // b
      8'hba,8'h78,8'h25,8'h2e,8'h1c,8'ha6,8'hb4,8'hc6,8'he8,8'hdd,8'h74,8'h1f,8'h4b,8'hbd,8'h8b,8'h8a, // c
      8'h70,8'h3e,8'hb5,8'h66,8'h48,8'h03,8'hf6,8'h0e,8'h61,8'h35,8'h57,8'hb9,8'h86,8'hc1,8'h1d,8'h9e, // d
      8'he1,8'hf8,8'h98,8'h11,8'h69,8'hd9,8'h8e,8'h94,8'h9b,8'h1e,8'h87,8'he9,8'hce,8'h55,8'h28,8'hdf, // e
      8'h8c,8'ha1,8'h89,8'h0d,8'hbf,8'he6,8'h42,8'h68,8'h41,8'h99,8'h2d,8'h0f,8'hb0,8'h54,8'hbb,8'h16  // f
    };
    assign SBOX_INV_LUT = {
        //  0	  1     2	  3     4     5     6     7     8     9     a     b     c     d     e     f
        8'h52,8'h09,8'h6a,8'hd5,8'h30,8'h36,8'ha5,8'h38,8'hbf,8'h40,8'ha3,8'h9e,8'h81,8'hf3,8'hd7,8'hfb, // 0	
        8'h7c,8'he3,8'h39,8'h82,8'h9b,8'h2f,8'hff,8'h87,8'h34,8'h8e,8'h43,8'h44,8'hc4,8'hde,8'he9,8'hcb, // 1	
        8'h54,8'h7b,8'h94,8'h32,8'ha6,8'hc2,8'h23,8'h3d,8'hee,8'h4c,8'h95,8'h0b,8'h42,8'hfa,8'hc3,8'h4e, // 2	
        8'h08,8'h2e,8'ha1,8'h66,8'h28,8'hd9,8'h24,8'hb2,8'h76,8'h5b,8'ha2,8'h49,8'h6d,8'h8b,8'hd1,8'h25, // 3	
        8'h72,8'hf8,8'hf6,8'h64,8'h86,8'h68,8'h98,8'h16,8'hd4,8'ha4,8'h5c,8'hcc,8'h5d,8'h65,8'hb6,8'h92, // 4	
        8'h6c,8'h70,8'h48,8'h50,8'hfd,8'hed,8'hb9,8'hda,8'h5e,8'h15,8'h46,8'h57,8'ha7,8'h8d,8'h9d,8'h84, // 5	
        8'h90,8'hd8,8'hab,8'h00,8'h8c,8'hbc,8'hd3,8'h0a,8'hf7,8'he4,8'h58,8'h05,8'hb8,8'hb3,8'h45,8'h06, // 6	
        8'hd0,8'h2c,8'h1e,8'h8f,8'hca,8'h3f,8'h0f,8'h02,8'hc1,8'haf,8'hbd,8'h03,8'h01,8'h13,8'h8a,8'h6b, // 7	
        8'h3a,8'h91,8'h11,8'h41,8'h4f,8'h67,8'hdc,8'hea,8'h97,8'hf2,8'hcf,8'hce,8'hf0,8'hb4,8'he6,8'h73, // 8	
        8'h96,8'hac,8'h74,8'h22,8'he7,8'had,8'h35,8'h85,8'he2,8'hf9,8'h37,8'he8,8'h1c,8'h75,8'hdf,8'h6e, // 9	
        8'h47,8'hf1,8'h1a,8'h71,8'h1d,8'h29,8'hc5,8'h89,8'h6f,8'hb7,8'h62,8'h0e,8'haa,8'h18,8'hbe,8'h1b, // a	
        8'hfc,8'h56,8'h3e,8'h4b,8'hc6,8'hd2,8'h79,8'h20,8'h9a,8'hdb,8'hc0,8'hfe,8'h78,8'hcd,8'h5a,8'hf4, // b	
        8'h1f,8'hdd,8'ha8,8'h33,8'h88,8'h07,8'hc7,8'h31,8'hb1,8'h12,8'h10,8'h59,8'h27,8'h80,8'hec,8'h5f, // c	
        8'h60,8'h51,8'h7f,8'ha9,8'h19,8'hb5,8'h4a,8'h0d,8'h2d,8'he5,8'h7a,8'h9f,8'h93,8'hc9,8'h9c,8'hef, // d	
        8'ha0,8'he0,8'h3b,8'h4d,8'hae,8'h2a,8'hf5,8'hb0,8'hc8,8'heb,8'hbb,8'h3c,8'h83,8'h53,8'h99,8'h61, // e	
        8'h17,8'h2b,8'h04,8'h7e,8'hba,8'h77,8'hd6,8'h26,8'he1,8'h69,8'h14,8'h63,8'h55,8'h21,8'h0c,8'h7d  // f	
    };

	wire[7:0] data_i;
	wire[7:0] sbox_fwd_o;
	wire[7:0] sbox_inv_o;

	sbox_fwd a(
		.data_i(data_i    ),
		.data_o(sbox_fwd_o)
	);
	sbox_inv b(
		.data_i(data_i    ),
		.data_o(sbox_inv_o)
	);

	integer i;
	initial begin
`ifdef __VOGLS__TWO_VALUE_LOGIC
        // Since `data_i = 0` would not cause update on tvl, it might
        // non-deterministically settle into the incorrect state.
        data_i = 1;
`endif
	    for (i = 0; i < 256; i = i + 1) begin
			#2
			data_i = i;
			#2
		    $vogls_assert_eq(sbox_fwd_o, SBOX_LUT    [2047 - 8*i-:8]);
		    $vogls_assert_eq(sbox_inv_o, SBOX_INV_LUT[2047 - 8*i-:8]);
		end
		$finish();
	end
endmodule
