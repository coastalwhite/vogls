module aes_mvn(vec, mat, data_o);
    input [7:0] vec;
    input [63:0] mat;
    output data_o;

    always @* begin
		data_o =
            (mat[0*8+2] & vec[7])
          ^ (mat[1*8+2] & vec[6])
          ^ (mat[2*8+2] & vec[5])
          ^ (mat[3*8+2] & vec[4])
          ^ (mat[4*8+2] & vec[3])
          ^ (mat[5*8+2] & vec[2])
          ^ (mat[6*8+2] & vec[1])
          ^ (mat[7*8+2] & vec[0]);
    end
endmodule

module tb();
    reg [7:0] vec;
    reg [63:0] mat;
    reg o;

    aes_mvn m(vec, mat, o);

    initial begin
        #1 mat = 64'hFF_A9_81_09_48_F2_F3_98; vec = 8'h00;
        #1 $vogls_assert_eq(o, 1'b0);
        
        #1 vec = 8'hFF;
        #1 $vogls_assert_eq(o, 1'b1);

        #1 vec = 8'h63;
        #1 $vogls_assert_eq(o, 1'b1);
    end
endmodule
