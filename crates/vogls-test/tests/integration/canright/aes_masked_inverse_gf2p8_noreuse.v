`ifndef NO_TB
module tb();
    reg  [7:0] i;
    wire [7:0] o;

    wire [256*8 - 1:0] LUT;
    assign LUT = {
        //  0	  1     2	  3     4     5     6     7     8     9     a     b     c     d     e     f
        8'h00,8'h60,8'hD0,8'hB0,8'hA0,8'h80,8'h10,8'hE0,8'h50,8'h90,8'h40,8'h30,8'hF0,8'h20,8'h70,8'hC0, // 0
        8'h06,8'hCC,8'h7E,8'h79,8'h2B,8'hA2,8'h28,8'hE3,8'h43,8'hF8,8'hF2,8'h5B,8'h59,8'h4E,8'hAC,8'hE6, // 1
        8'h0D,8'hE7,8'h88,8'hE9,8'hF7,8'h9D,8'hC9,8'h58,8'h16,8'h92,8'h51,8'h14,8'hC2,8'hF6,8'hA4,8'hA1, // 2
        8'h0B,8'h97,8'h9E,8'h44,8'h81,8'h53,8'hAD,8'h5C,8'hAE,8'hF4,8'h7B,8'h87,8'h3D,8'h3C,8'h71,8'hF3, // 3
        8'h0A,8'hB2,8'h7F,8'h18,8'h33,8'h8D,8'hEB,8'h74,8'hC8,8'hBF,8'hE2,8'h8A,8'hC4,8'h9B,8'h1D,8'h93, // 4
        8'h08,8'h2A,8'hD9,8'h35,8'hD8,8'hAA,8'hF9,8'h75,8'h27,8'h1C,8'h6D,8'h1B,8'h37,8'h7C,8'hFD,8'h6B, // 5
        8'h01,8'h82,8'h9C,8'hDA,8'hBE,8'h9F,8'h77,8'h68,8'h67,8'hB8,8'h8E,8'h5F,8'hFC,8'h5A,8'hF1,8'hD2, // 6
        8'h0E,8'h3E,8'h85,8'hC5,8'h47,8'h57,8'h86,8'h66,8'hB3,8'h13,8'hBA,8'h3A,8'h5D,8'hCD,8'h12,8'h42, // 7
        8'h05,8'h34,8'h61,8'hEA,8'h8C,8'h72,8'h76,8'h3B,8'h22,8'hEC,8'h4B,8'h96,8'h84,8'h45,8'h6A,8'h91, // 8
        8'h09,8'h8F,8'h29,8'h4F,8'hFB,8'hC1,8'h8B,8'h31,8'hCE,8'hDD,8'hFE,8'h4D,8'h62,8'h25,8'h32,8'h65, // 9
        8'h04,8'h2F,8'h15,8'hB7,8'h2E,8'hD6,8'hE8,8'hAB,8'hB4,8'hEF,8'h55,8'hA7,8'h1E,8'h36,8'h38,8'hDB, // a
        8'h03,8'hB5,8'h41,8'h78,8'hA8,8'hB1,8'hF5,8'hA3,8'h69,8'hD4,8'h7A,8'hEE,8'hDE,8'hFA,8'h64,8'h49, // b
        8'h0F,8'h95,8'h2C,8'hD3,8'h4C,8'h73,8'hCF,8'hD5,8'h48,8'h26,8'hE1,8'hED,8'h11,8'h7D,8'h98,8'hC6, // c
        8'h02,8'hE4,8'h6F,8'hC3,8'hB9,8'hC7,8'hA5,8'hDC,8'h54,8'h52,8'h63,8'hAF,8'hD7,8'h99,8'hBC,8'hE5, // d
        8'h07,8'hCA,8'h4A,8'h17,8'hD1,8'hDF,8'h1F,8'h21,8'hA6,8'h23,8'h83,8'h46,8'h89,8'hCB,8'hBB,8'hA9, // e
        8'h0C,8'h6E,8'h1A,8'h3F,8'h39,8'hB6,8'h2D,8'h24,8'h19,8'h56,8'hBD,8'h94,8'h6C,8'h5E,8'h9A,8'hFF  // f
    };
    wire [3*10-1:0] PRD;
    wire [3*8-1 :0] MLUT, NLUT;
    assign PRD  = { 10'h000, 10'h3FF, 10'h3AB };
    assign MLUT = { 8'h00, 8'hAB, 8'h63 };
    assign NLUT = { 8'h00, 8'h63, 8'hCB };

    reg [7:0] m, n;
    reg [9:0] prd;
    aes_masked_inverse_gf2p8_noreuse m1(i, m, n, prd, o);

	integer j, k;
	initial begin
        #0
        for (k = 0; k < 3; k = k + 1) begin
            prd = PRD[k*10 +: 10];
            m   = MLUT[k*8 +:  8];
            n   = NLUT[k*8 +:  8];
            for (j = 0; j <= 255; j = j + 1) begin
                #1 i = j ^ m;
                #1 $vogls_assert_eq(o ^ n, LUT[2047 - j*8 -: 8]);
            end
        end
    end
endmodule
`endif

`define NO_TB
`include "./aes_mul_gf2p4.v"
`include "./aes_masked_inverse_gf2p4_noreuse.v"
`include "./aes_square_scale_gf2p4_gf2p2.v"

module aes_masked_inverse_gf2p8_noreuse(
    input  [7:0] a,
    input  [7:0] m,
    input  [7:0] n,
    input  [9:0] prd,
    output [7:0] a_inv
);
    wire [3:0] a1, a0, m1, m0;
    assign a1 = a[7:4];
    assign a0 = a[3:0];
    assign m1 = m[7:4];
    assign m0 = m[3:0];

    wire [1:0] r;
    wire [3:0] q, t, s1, s0;
    assign r = prd[1:0];
    assign q = prd[5:2];
    assign t = prd[9:6];
    assign s1 = n[7:4];
    assign s0 = n[3:0];

    wire [3:0] ss_a1_a0, ss_m1_m0;
    aes_square_scale_gf2p4_gf2p2 blk0_sqsc0(.gamma(a1 ^ a0), .delta(ss_a1_a0));
    aes_square_scale_gf2p4_gf2p2 blk0_sqsc1(.gamma(m1 ^ m0), .delta(ss_m1_m0));

    wire [3:0] mul_a1_a0, mul_a1_m0, mul_a0_m1, mul_m0_m1;
    aes_mul_gf2p4 blk1_m0(.gamma(a1), .delta(a0), .theta(mul_a1_a0));
    aes_mul_gf2p4 blk1_m1(.gamma(a1), .delta(m0), .theta(mul_a1_m0));
    aes_mul_gf2p4 blk1_m2(.gamma(a0), .delta(m1), .theta(mul_a0_m1));
    aes_mul_gf2p4 blk1_m3(.gamma(m0), .delta(m1), .theta(mul_m0_m1));

    wire [3:0] b [5:0];
    assign b[0] = q ^ ss_a1_a0; // q does not depend on a1, a0.
    assign b[1] = b[0] ^ ss_m1_m0; // b[0] does not depend on m1, m0.
    assign b[2] = b[1] ^ mul_a1_a0;
    assign b[3] = b[2] ^ mul_a1_m0;
    assign b[4] = b[3] ^ mul_a0_m1;
    assign b[5] = b[4] ^ mul_m0_m1;

    wire [3:0] b_inv;
    aes_masked_inverse_gf2p4_noreuse blk2_inv(.b(b[5]), .q(q), .r(r), .t(t), .b_inv(b_inv));

    wire [3:0] mul_a0_b_inv, mul_a0_t, mul_m0_b_inv, mul_m0_t, mul_a1_b_inv, mul_a1_t, mul_m1_b_inv, mul_m1_t;
    aes_mul_gf2p4 blk3_m0(.gamma(a0), .delta(b_inv), .theta(mul_a0_b_inv));
    aes_mul_gf2p4 blk3_m1(.gamma(a0), .delta(t    ), .theta(mul_a0_t    ));
    aes_mul_gf2p4 blk3_m2(.gamma(m0), .delta(b_inv), .theta(mul_m0_b_inv));
    aes_mul_gf2p4 blk3_m3(.gamma(m0), .delta(t    ), .theta(mul_m0_t    ));
    aes_mul_gf2p4 blk3_m4(.gamma(a1), .delta(b_inv), .theta(mul_a1_b_inv));
    aes_mul_gf2p4 blk3_m5(.gamma(a1), .delta(t    ), .theta(mul_a1_t    ));
    aes_mul_gf2p4 blk3_m6(.gamma(m1), .delta(b_inv), .theta(mul_m1_b_inv));
    aes_mul_gf2p4 blk3_m7(.gamma(m1), .delta(t    ), .theta(mul_m1_t    ));

    wire [3:0] a1_inv [3:0], a0_inv [3:0];
    assign a1_inv[0] = s1 ^ mul_a0_b_inv;
    assign a1_inv[1] = a1_inv[0] ^ mul_a0_t;
    assign a1_inv[2] = a1_inv[1] ^ mul_m0_b_inv;
    assign a1_inv[3] = a1_inv[2] ^ mul_m0_t;
    assign a0_inv[0] = s0 ^ mul_a1_b_inv;
    assign a0_inv[1] = a0_inv[0] ^ mul_a1_t;
    assign a0_inv[2] = a0_inv[1] ^ mul_m1_b_inv;
    assign a0_inv[3] = a0_inv[2] ^ mul_m1_t;

    assign a_inv = { a1_inv[3], a0_inv[3] };
endmodule
