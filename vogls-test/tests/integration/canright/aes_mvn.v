`ifndef NO_TB
`define A2X 64'hFF_A9_81_09_48_F2_F3_98
`define S2X 64'h53_51_04_12_EB_05_79_8C

module tb();
    reg [7:0] vec;
    reg [63:0] mat;
    reg [7:0] o;

    aes_mvn m(vec, mat, o);

    initial begin
        #1 mat = `A2X; vec = 8'h00;
        #1 $vogls_assert_eq(o, 8'h00);
        
        #1 vec = 8'hFF;
        #1 $vogls_assert_eq(o, 8'h0F);

        #1 vec = 8'h63;
        #1 $vogls_assert_eq(o, 8'h57);

        #1 mat = `S2X; vec = 8'h00;
        #1 $vogls_assert_eq(o, 8'h00);

        #1 vec = 8'hFF;
        #1 $vogls_assert_eq(o, 8'h0F);

        #1 vec = 8'h63;
        #1 $vogls_assert_eq(o, 8'h7E);

        #1 vec = 8'h88;
        #1 $vogls_assert_eq(o, 8'h9E);

        #1 vec = 8'hA1;
        #1 $vogls_assert_eq(o, 8'hDA);
    end
endmodule
`endif

module aes_mvn(vec, mat, data_o);
    input  [7:0] vec;
    input [63:0] mat;
    output [7:0] data_o;
    assign data_o = {
          (mat[0*8+7] & vec[7-0]) ^
        ^ (mat[1*8+7] & vec[7-1]) ^
        ^ (mat[2*8+7] & vec[7-2]) ^
        ^ (mat[3*8+7] & vec[7-3]) ^
        ^ (mat[4*8+7] & vec[7-4]) ^
        ^ (mat[5*8+7] & vec[7-5]) ^
        ^ (mat[6*8+7] & vec[7-6]) ^
        ^ (mat[7*8+7] & vec[7-7]),

          (mat[0*8+6] & vec[7-0]) ^
        ^ (mat[1*8+6] & vec[7-1]) ^
        ^ (mat[2*8+6] & vec[7-2]) ^
        ^ (mat[3*8+6] & vec[7-3]) ^
        ^ (mat[4*8+6] & vec[7-4]) ^
        ^ (mat[5*8+6] & vec[7-5]) ^
        ^ (mat[6*8+6] & vec[7-6]) ^
        ^ (mat[7*8+6] & vec[7-7]),

          (mat[0*8+5] & vec[7-0]) ^
        ^ (mat[1*8+5] & vec[7-1]) ^
        ^ (mat[2*8+5] & vec[7-2]) ^
        ^ (mat[3*8+5] & vec[7-3]) ^
        ^ (mat[4*8+5] & vec[7-4]) ^
        ^ (mat[5*8+5] & vec[7-5]) ^
        ^ (mat[6*8+5] & vec[7-6]) ^
        ^ (mat[7*8+5] & vec[7-7]),

          (mat[0*8+4] & vec[7-0]) ^
        ^ (mat[1*8+4] & vec[7-1]) ^
        ^ (mat[2*8+4] & vec[7-2]) ^
        ^ (mat[3*8+4] & vec[7-3]) ^
        ^ (mat[4*8+4] & vec[7-4]) ^
        ^ (mat[5*8+4] & vec[7-5]) ^
        ^ (mat[6*8+4] & vec[7-6]) ^
        ^ (mat[7*8+4] & vec[7-7]),

          (mat[0*8+3] & vec[7-0]) ^
        ^ (mat[1*8+3] & vec[7-1]) ^
        ^ (mat[2*8+3] & vec[7-2]) ^
        ^ (mat[3*8+3] & vec[7-3]) ^
        ^ (mat[4*8+3] & vec[7-4]) ^
        ^ (mat[5*8+3] & vec[7-5]) ^
        ^ (mat[6*8+3] & vec[7-6]) ^
        ^ (mat[7*8+3] & vec[7-7]),

          (mat[0*8+2] & vec[7-0]) ^
        ^ (mat[1*8+2] & vec[7-1]) ^
        ^ (mat[2*8+2] & vec[7-2]) ^
        ^ (mat[3*8+2] & vec[7-3]) ^
        ^ (mat[4*8+2] & vec[7-4]) ^
        ^ (mat[5*8+2] & vec[7-5]) ^
        ^ (mat[6*8+2] & vec[7-6]) ^
        ^ (mat[7*8+2] & vec[7-7]),

          (mat[0*8+1] & vec[7-0]) ^
        ^ (mat[1*8+1] & vec[7-1]) ^
        ^ (mat[2*8+1] & vec[7-2]) ^
        ^ (mat[3*8+1] & vec[7-3]) ^
        ^ (mat[4*8+1] & vec[7-4]) ^
        ^ (mat[5*8+1] & vec[7-5]) ^
        ^ (mat[6*8+1] & vec[7-6]) ^
        ^ (mat[7*8+1] & vec[7-7]),

          (mat[0*8+0] & vec[7-0]) ^
        ^ (mat[1*8+0] & vec[7-1]) ^
        ^ (mat[2*8+0] & vec[7-2]) ^
        ^ (mat[3*8+0] & vec[7-3]) ^
        ^ (mat[4*8+0] & vec[7-4]) ^
        ^ (mat[5*8+0] & vec[7-5]) ^
        ^ (mat[6*8+0] & vec[7-6]) ^
        ^ (mat[7*8+0] & vec[7-7])
    };
endmodule
