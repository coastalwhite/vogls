`ifndef NO_TB
module tb();
    reg [3:0] gamma, delta;
    wire [3:0] theta;

    aes_mul_gf2p4 m(gamma, delta, theta);

    initial begin
        #1 gamma = 4'h0; delta = 4'h0;
        #1 $vogls_assert_eq(theta, 4'h0);
        
        #1 gamma = 4'hF; delta = 4'h0;
        #1 $vogls_assert_eq(theta, 4'h0);
        
        #1 gamma = 4'hF; delta = 4'hF;
        #1 $vogls_assert_eq(theta, 4'hF);
        
        #1 gamma = 4'h0; delta = 4'hF;
        #1 $vogls_assert_eq(theta, 4'h0);
        
        #1 gamma = 4'hA; delta = 4'h6;
        #1 $vogls_assert_eq(theta, 4'hD);
        
        #1 gamma = 4'h7; delta = 4'h9;
        #1 $vogls_assert_eq(theta, 4'h8);
        
        #1 gamma = 4'h7; delta = 4'h1;
        #1 $vogls_assert_eq(theta, 4'hB);
    end
endmodule
`endif

`define NO_TB
`include "./aes_mul_gf2p2.v"
`include "./aes_scale_omega2_gf2p2.v"

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

