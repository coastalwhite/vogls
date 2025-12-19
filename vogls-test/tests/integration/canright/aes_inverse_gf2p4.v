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

    aes_inverse_gf2p4 m(i, o);

    integer j;
    initial begin
        for (j = 0; j < 16; j = j + 1) begin
            #1 i = j;
            #1 $vogls_assert_eq(o, LUT[63 - j*4 -: 4]);
        end
    end
endmodule
`endif

`define NO_TB
`include "./aes_mul_gf2p2.v"
`include "./aes_scale_omega2_gf2p2.v"
`include "./aes_square_gf2p2.v"

module aes_inverse_gf2p4(data_i, data_o);
    input  [3:0] data_i;
    output [3:0] data_o;

    wire [1:0] a, b, c, c1, d;

    assign a = data_i[3:2] ^ data_i[1:0];

    aes_mul_gf2p2            m1( .a_i(data_i[3:2]), .b_i(data_i[1:0]), .z_i(b)     );
    aes_square_gf2p2       sqc1( .data_i(a),                           .data_o(c1) );
    aes_scale_omega2_gf2p2   sc( .data_i(c1),                          .data_o(c)  );
    aes_square_gf2p2        inv( .data_i(c ^ b),                       .data_o(d)  );

    aes_mul_gf2p2          m2( .a_i(d), .b_i(data_i[1:0]), .z_o(data_o[3:2]) );
    aes_mul_gf2p2          m3( .a_i(d), .b_i(data_i[3:2]), .z_o(data_o[1:0]) );
endmodule
