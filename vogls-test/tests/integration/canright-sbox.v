`define A2X 64'hFF_A9_81_09_48_F2_F3_98
`define X2A 64'h60_DE_29_68_8C_6E_78_64
`define X2S 64'h24_03_04_DC_0B_9E_2D_58
`define S2X 64'h53_51_04_12_EB_05_79_8C

module aes_mvn(vec, mat, data_o);
    input [7:0] vec;
    input [63:0] mat;
    output [7:0] data_o;

    wire  [7:0] c0, c1, c2, c3, c4, c5, c6, c7;

    integer j;
    always begin
		for (j = 0; j < 8; j = j + 1) begin
			c0[j] = mat[j*8+0] & vec[7 - j];
			c1[j] = mat[j*8+1] & vec[7 - j];
			c2[j] = mat[j*8+2] & vec[7 - j];
			c3[j] = mat[j*8+3] & vec[7 - j];
			c4[j] = mat[j*8+4] & vec[7 - j];
			c5[j] = mat[j*8+5] & vec[7 - j];
			c6[j] = mat[j*8+6] & vec[7 - j];
			c7[j] = mat[j*8+7] & vec[7 - j];
		end
		data_o = { ^c7, ^c6, ^c5, ^c4, ^c3, ^c2, ^c1, ^c0 };
    end
endmodule

module aes_mul_gf2p2(a_i, b_i, z_o);
    input [1:0]  a_i;
    input [1:0]  b_i;
    output [1:0] z_o;

    wire a, b, c;

    assign a = a_i[1] & b_i[1];
    assign b = ^a_i & ^b_i;
    assign c = a_i[0] & b_i[0];

    assign z_o = { a ^ b, c ^ b };
endmodule

module aes_scale_omega2_gf2p2(data_i, data_o);
    input  [1:0] data_i;
    output [1:0] data_o;

    assign data_o = { data_i[0], ^data_i };
endmodule

module aes_scale_omega_gf2p2(data_i, data_o);
    input  [1:0] data_i;
    output [1:0] data_o;

    assign data_o = { ^data_i, data_i[0] };
endmodule

module aes_square_gf2p2(data_i, data_o);
    input [1:0] data_i;
    input [1:0] data_o;

    assign data_o = { data_i[0], data_i[1] };
endmodule

module aes_mul_gf2p4(gamma, delta, theta);
    input [3:0]  gamma;
    input [3:0]  delta;
    output [3:0] theta;

    wire [1:0] a, b, c, t;

    aes_mul_gf2p2 m1(
        .a_i(gamma[3:2]),
        .b_i(delta[3:2]),
        .z_i(a)
    );
    aes_mul_gf2p2 m2(
        .a_i(gamma[3:2] ^ gamma[1:0]),
        .b_i(delta[3:2] ^ delta[1:0]),
        .z_i(b)
    );
    aes_mul_gf2p2 m3(
        .a_i(gamma[1:0]),
        .b_i(delta[1:0]),
        .z_i(c)
    );
    aes_scale_omega2_gf2p2 sc1(
        .data_i(b),
        .data_o(t)
    );
    
    assign theta = { a ^ t, c ^ t };
endmodule

module aes_square_scale_gf2p4_gf2p2(data_i, data_o);
    input [3:0]  data_i;
    output [3:0] data_o;

    wire [1:0] a, b;

    assign a = data_i[3:2] ^ data_i[1:0];
    aes_square_gf2p2 sq1(
        .data_i(data_i[1:0]),
        .data_o(b)
    );

    aes_square_gf2p2 sq2(
        .data_i(a),
        .data_o(data_o[3:2])
    );
    aes_scale_omega_gf2p2 sc(
        .data_i(b),
        .data_o(data_o[1:0])
    );
endmodule

module aes_inverse_gf2p4(data_i, data_o);
    input [3:0]  data_i;
    output [3:0] data_o;

    wire [1:0] a, b, c, c1, d;

    assign a = data_i[3:2] ^ data_i[1:0];

    aes_mul_gf2p2 m1(
        .a_i(data_i[3:2]),
        .b_i(data_i[1:0]),
        .z_i(b)
    );
    aes_square_gf2p2 sqc1(
        .data_i(a),
        .data_o(c1)
    );
    aes_scale_omega2_gf2p2 sc(
        .data_i(c1),
        .data_o(c)
    );
    aes_square_gf2p2 inv(
        .data_i(c ^ b),
        .data_o(d)
    );

    aes_mul_gf2p2 m2(
        .a_i(d          ),
        .b_i(data_i[1:0] ),
        .z_o(data_o[3:2])
    );
    aes_mul_gf2p2 m3(
        .a_i(d          ),
        .b_i(data_i[3:2]),
        .z_o(data_o[1:0])
    );
endmodule

module aes_inverse_gf2p8(data_i, data_o);
    input [7:0]  data_i;
    output [7:0] data_o;

    wire [3:0] a, b, c, d;

    assign a = data_i[7:4] ^ data_i[3:0];

    aes_mul_gf2p4 m1(
        .gamma(data_i[7:4]),
        .delta(data_i[3:0]),
        .theta(b)
    );
    aes_square_scale_gf2p4_gf2p2 sqsc(
        .data_i(a),
        .data_o(c)
    );
    aes_inverse_gf2p4 inv(
        .data_i(c ^ b),
        .data_o(d)
    );

    aes_mul_gf2p4 m2(
        .gamma(d          ),
        .delta(data_i[3:0] ),
        .theta(data_o[7:4])
    );
    aes_mul_gf2p4 m3(
        .gamma(d          ),
        .delta(data_i[7:4] ),
        .theta(data_o[3:0])
    );
endmodule

module sbox_fwd(data_i, data_o);
    input [7:0]  data_i;
    output [7:0] data_o;

    wire [7:0] data_basis_x;
    wire [7:0] data_inverse;
    wire [7:0] data_basis_s;

    always begin #1 $display("hi!"); $display(data_basis_s); end

    assign data_o = data_basis_s ^ 8'h63;

    aes_mvn a2x(
        .vec   (data_i      ),
        .matrix(`A2X         ),
        .data_o(data_basis_x)
    );
    aes_inverse_gf2p8 inv(
        .data_i(data_basis_x),
        .data_o(data_inverse)
    );
    aes_mvn x2s(
        .vec   (data_inverse),
        .matrix(`X2S),
        .data_o(data_basis_s)
    );
endmodule

module sbox_inv(data_i, data_o);
    input [7:0]  data_i;
    output [7:0] data_o;

    wire [7:0] data_basis_x;
    wire [7:0] data_inverse;

    assign data_o = data_basis_s ^ 8'h63;

    aes_mvn s2x(
        .vec   (data_i & 8'h63),
        .matrix(`S2X           ),
        .data_o(data_basis_x  )
    );
    aes_inverse_gf2p8 inv(
        .data_i(data_basis_x),
        .data_o(data_inverse)
    );
    aes_mvn x2a(
        .vec   (data_inverse),
        .matrix(`X2A),
        .data_o(data_basis_s)
    );
endmodule


module tb();
	wire [ 2047/*256 * 8 - 1*/ :0] SBOX_LUT;
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

	wire [7:0] data_i;
	wire[7:0] sbox_fwd_o;
	wire[7:0] sbox_inv_o;

	sbox_fwd a(
		.data_i(data_i    ),
		.data_o(sbox_fwd_o)
	);
	sbox_fwd b(
		.data_i(data_i    ),
		.data_o(sbox_inv_o)
	);

	integer i;
	initial begin
	    for (i = 0; i <= 1; i = i + 1) begin
			#2
			data_i = i;
            $display(data_i);
            $display(SBOX_LUT[2047 - 8*i-:8]);
            $display(sbox_fwd_o);
			#2
		    $vogls_assert_eq(sbox_fwd_o, SBOX_LUT[2047 - 8*i-:8]);
		end
	end
//
// }
//
// #[test]
// pub fn test_sbox_inv() {
//     #[rustfmt::skip]
//     static SBOX_INV_LUT: [u8; 256] = [
//         // 0	1    2	  3    4    5    6    7    8    9    a    b    c    d    e    f
//         0x52,0x09,0x6a,0xd5,0x30,0x36,0xa5,0x38,0xbf,0x40,0xa3,0x9e,0x81,0xf3,0xd7,0xfb, // 0	
//         0x7c,0xe3,0x39,0x82,0x9b,0x2f,0xff,0x87,0x34,0x8e,0x43,0x44,0xc4,0xde,0xe9,0xcb, // 1	
//         0x54,0x7b,0x94,0x32,0xa6,0xc2,0x23,0x3d,0xee,0x4c,0x95,0x0b,0x42,0xfa,0xc3,0x4e, // 2	
//         0x08,0x2e,0xa1,0x66,0x28,0xd9,0x24,0xb2,0x76,0x5b,0xa2,0x49,0x6d,0x8b,0xd1,0x25, // 3	
//         0x72,0xf8,0xf6,0x64,0x86,0x68,0x98,0x16,0xd4,0xa4,0x5c,0xcc,0x5d,0x65,0xb6,0x92, // 4	
//         0x6c,0x70,0x48,0x50,0xfd,0xed,0xb9,0xda,0x5e,0x15,0x46,0x57,0xa7,0x8d,0x9d,0x84, // 5	
//         0x90,0xd8,0xab,0x00,0x8c,0xbc,0xd3,0x0a,0xf7,0xe4,0x58,0x05,0xb8,0xb3,0x45,0x06, // 6	
//         0xd0,0x2c,0x1e,0x8f,0xca,0x3f,0x0f,0x02,0xc1,0xaf,0xbd,0x03,0x01,0x13,0x8a,0x6b, // 7	
//         0x3a,0x91,0x11,0x41,0x4f,0x67,0xdc,0xea,0x97,0xf2,0xcf,0xce,0xf0,0xb4,0xe6,0x73, // 8	
//         0x96,0xac,0x74,0x22,0xe7,0xad,0x35,0x85,0xe2,0xf9,0x37,0xe8,0x1c,0x75,0xdf,0x6e, // 9	
//         0x47,0xf1,0x1a,0x71,0x1d,0x29,0xc5,0x89,0x6f,0xb7,0x62,0x0e,0xaa,0x18,0xbe,0x1b, // a	
//         0xfc,0x56,0x3e,0x4b,0xc6,0xd2,0x79,0x20,0x9a,0xdb,0xc0,0xfe,0x78,0xcd,0x5a,0xf4, // b	
//         0x1f,0xdd,0xa8,0x33,0x88,0x07,0xc7,0x31,0xb1,0x12,0x10,0x59,0x27,0x80,0xec,0x5f, // c	
//         0x60,0x51,0x7f,0xa9,0x19,0xb5,0x4a,0x0d,0x2d,0xe5,0x7a,0x9f,0x93,0xc9,0x9c,0xef, // d	
//         0xa0,0xe0,0x3b,0x4d,0xae,0x2a,0xf5,0xb0,0xc8,0xeb,0xbb,0x3c,0x83,0x53,0x99,0x61, // e	
//         0x17,0x2b,0x04,0x7e,0xba,0x77,0xd6,0x26,0xe1,0x69,0x14,0x63,0x55,0x21,0x0c,0x7d, // f	
//     ];
//
//     for i in 0..=255u8 {
//         assert_eq!(sbox_inv(i), SBOX_INV_LUT[i as usize]);
//     }
endmodule
