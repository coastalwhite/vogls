module top_module ( 
    input [7:0] in, 
    output [2:0] pos
);
    always @(*) begin
        casez (in)
			8'bzzzz_zzz1: pos = 0;
			8'bzzzz_zz10: pos = 1;
			8'bzzzz_z100: pos = 2;
			8'bzzzz_1000: pos = 3;
			8'bzzz1_0000: pos = 4;
			8'bzz10_0000: pos = 5;
			8'bz100_0000: pos = 6;
			8'b1000_0000: pos = 7;
			default:      pos = 0;
        endcase
    end
endmodule

module tb();
    reg [7:0] in;
    wire [2:0] pos;

    top_module m(in, pos);

    integer i = 0;
    initial begin
        in = 4'h0;
        #1 $vogls_assert_eq(pos, 0); in = 8'h1; 
        #1 $vogls_assert_eq(pos, 0); in = 8'h4; 
        #1 $vogls_assert_eq(pos, 2);

        for (i = 0; i < 16; i = i + 1) begin
            in = i;
            #1 $vogls_assert_eq(pos, 
                i[0] ? 0 : (
                    i[1] ? 1 : (
                        i[2] ? 2 : (
                            i[3] ? 3 : (
								i[4] ? 4 : (
									i[5] ? 5 : (
										i[6] ? 6 : (
											i[7] ? 7 : 0
										)
									)
								)
							)
                        )
                    )
                )
            );
        end
    end
endmodule
