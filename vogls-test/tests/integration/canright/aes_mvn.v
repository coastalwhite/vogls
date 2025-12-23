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

    wire  [7:0] c0, c1, c2, c3, c4, c5, c6, c7;

    integer j;
    always @* begin
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
