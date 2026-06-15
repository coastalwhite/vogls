`ifndef NO_TB
module tb();
    reg [3:0] gamma;
    wire [3:0] delta;

    wire [4*16-1:0] LUT;
    assign LUT = {
        4'h0, 4'hB, 4'h6, 4'hD,
        4'h8, 4'h3, 4'hE, 4'h5,
        4'h4, 4'hF, 4'h2, 4'h9,
        4'hC, 4'h7, 4'hA, 4'h1
    };

    aes_square_scale_gf2p4_gf2p2 s(gamma, delta);

    integer i;
    initial begin
	    for (i = 0; i <= 15; i = i + 1) begin
            #1 gamma = i;
            #1 $vogls_assert_eq(delta, LUT[63 - 4*i -: 4]);
        end
    end
endmodule
`endif

`define NO_TB
`include "./aes_square_gf2p2.v"
`include "./aes_scale_omega_gf2p2.v"

module aes_square_scale_gf2p4_gf2p2(gamma, delta);
    input  [3:0] gamma;
    output [3:0] delta;

    wire [1:0] a, b, t1, t2;

    assign a = gamma[3:2] ^ gamma[1:0];
    assign delta = { t1, t2 };

    aes_square_gf2p2      sq1( .data_i(gamma[1:0]), .data_o(b)  );
    aes_square_gf2p2      sq2( .data_i(a),          .data_o(t1) );
    aes_scale_omega_gf2p2 sc ( .data_i(b),          .data_o(t2) );
endmodule
