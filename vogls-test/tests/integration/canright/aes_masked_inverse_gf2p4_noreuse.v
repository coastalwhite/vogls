`ifndef NO_TB
module tb();
    reg  [3:0] i;
    wire [3:0] o;

    wire [4*16-1:0] LUT;
    assign LUT = {
        4'h0, 4'hC, 4'h8, 4'h4,
        4'h3, 4'hA, 4'h7, 4'h6,
        4'h2, 4'hD, 4'h5, 4'hE,
        4'h1, 4'h9, 4'hB, 4'hF
    };
    wire [3*10-1:0] PRD;
    assign PRD = {
        10'h000,
        10'h3FF,
        10'h3AB
    };

    reg [3:0] q;
    reg [1:0] r;
    reg [3:0] t;

    aes_masked_inverse_gf2p4_noreuse m(i, q, r, t, o);

    integer j, k;
    initial begin
        for (k = 0; k < 3; k = k + 1) begin
            q = PRD[k*10+2 +: 4];
            r = PRD[k*10+0 +: 2];
            t = PRD[k*10+6 +: 4];

            for (j = 0; j < 16; j = j + 1) begin
                #1 i = j;
                #1 $vogls_assert_eq(o, LUT[63 - j*4 -: 4]);
            end
        end
    end
endmodule
`endif

`define NO_TB
`include "./aes_mul_gf2p2.v"
`include "./aes_scale_omega2_gf2p2.v"
`include "./aes_square_gf2p2.v"

module aes_masked_inverse_gf2p4_noreuse(
    input  [3:0] b,
    input  [3:0] q,
    input  [1:0] r,
    input  [3:0] t,
    output [3:0] b_inv
);
    wire [1:0] b1, b0, q1, q0, t1, t0;

    assign b1 = b[3:2];
    assign b0 = b[1:0];
    assign q1 = q[3:2];
    assign q0 = q[1:0];
    assign t1 = t[3:2];
    assign t0 = t[1:0];

    wire [1:0] scale_omega2_b, scale_omega2_q;
    wire [1:0] blk0_t0, blk0_t1;
    wire [1:0] mul_b1_b0, mul_b1_q0, mul_b0_q1, mul_q1_q0;

    // scale_omega2_b = aes_scale_omega2_gf2p2(aes_square_gf2p2(b1 ^ b0));
    aes_square_gf2p2         blk0_sq0(.data_i(b1 ^ b0), .data_o(blk0_t0));
    aes_scale_omega2_gf2p2 blk0_scom0(.data_i(blk0_t0), .data_o(scale_omega2_b));
    // scale_omega2_q = aes_scale_omega2_gf2p2(aes_square_gf2p2(q1 ^ q0));
    aes_square_gf2p2         blk0_sq1(.data_i(q1 ^ q0), .data_o(blk0_t1));
    aes_scale_omega2_gf2p2 blk0_scom1(.data_i(blk0_t1), .data_o(scale_omega2_q));
    aes_mul_gf2p2 blk0_m0(.a_i(b1), .b_i(b0), .z_o(mul_b1_b0));
    aes_mul_gf2p2 blk0_m1(.a_i(b1), .b_i(q0), .z_o(mul_b1_q0));
    aes_mul_gf2p2 blk0_m2(.a_i(b0), .b_i(q1), .z_o(mul_b0_q1));
    aes_mul_gf2p2 blk0_m3(.a_i(q1), .b_i(q0), .z_o(mul_q1_q0));

    wire [1:0] c [5:0];
    assign c[0] = r ^ scale_omega2_b;
    assign c[1] = c[0] ^ scale_omega2_q;
    assign c[2] = c[1] ^ mul_b1_b0;
    assign c[3] = c[2] ^ mul_b1_q0;
    assign c[4] = c[3] ^ mul_b0_q1;
    assign c[5] = c[4] ^ mul_q1_q0;

    wire [1:0] c_inv, r_sq;
    aes_square_gf2p2 blk2_sq0(.data_i(c[5]), .data_o(c_inv));
    aes_square_gf2p2 blk2_sq1(.data_i(r),    .data_o(r_sq ));

    wire [1:0] mul_b0_r_sq, mul_q0_c_inv, mul_q0_r_sq, mul_b1_r_sq, mul_q1_c_inv, mul_q1_r_sq;
    aes_mul_gf2p2 blk3_m0(.a_i(b0), .b_i(r_sq ), .z_o(mul_b0_r_sq ));
    aes_mul_gf2p2 blk3_m1(.a_i(q0), .b_i(c_inv), .z_o(mul_q0_c_inv));
    aes_mul_gf2p2 blk3_m2(.a_i(q0), .b_i(r_sq ), .z_o(mul_q0_r_sq ));
    aes_mul_gf2p2 blk3_m3(.a_i(b1), .b_i(r_sq ), .z_o(mul_b1_r_sq ));
    aes_mul_gf2p2 blk3_m4(.a_i(q1), .b_i(c_inv), .z_o(mul_q1_c_inv));
    aes_mul_gf2p2 blk3_m5(.a_i(q1), .b_i(r_sq ), .z_o(mul_q1_r_sq ));

    wire [1:0] b1_inv [3:0], b0_inv [3:0];
    wire [1:0] blk4_t0, blk4_t1;
    aes_mul_gf2p2 blk4_m0(.a_i(b0), .b_i(c_inv), .z_o(blk4_t0));
    assign b1_inv[0] = t1 ^ blk4_t0; // t1 does not depend on b0, c_inv.
    assign b1_inv[1] = b1_inv[0] ^ mul_b0_r_sq;
    assign b1_inv[2] = b1_inv[1] ^ mul_q0_c_inv;
    assign b1_inv[3] = b1_inv[2] ^ mul_q0_r_sq;
    aes_mul_gf2p2 blk4_m1(.a_i(b1), .b_i(c_inv), .z_o(blk4_t1));
    assign b0_inv[0] = t0 ^ blk4_t1; // t0 does not depend on b1, c_inv.
    assign b0_inv[1] = b0_inv[0] ^ mul_b1_r_sq;
    assign b0_inv[2] = b0_inv[1] ^ mul_q1_c_inv;
    assign b0_inv[3] = b0_inv[2] ^ mul_q1_r_sq;

    assign b_inv = { b1_inv[3], b0_inv[3] };
endmodule
