module top_module ( 
    input [3:0] in, 
    output [1:0] pos
);
    always @(*) begin
        case (in)
            4'b0000, // Zero falls through to 0.
            4'b0001, 4'b0011,
            4'b0101, 4'b0111,
            4'b1001, 4'b1011,
            4'b1101, 4'b1111: pos = 0;
            4'b0010, 4'b0110,
            4'b1010, 4'b1110: pos = 1;
            4'b0100, 4'b1100: pos = 2;
            4'b1000         : pos = 3;
        endcase
    end

endmodule

module tb();
    reg [3:0] in;
    wire [1:0] pos;

    top_module m(in, pos);

    integer i = 0;
    initial begin
        in = 4'h0;
        #1 $vogls_assert_eq(pos, 0); in = 4'h1; 
        #1 $vogls_assert_eq(pos, 0); in = 4'h4; 
        #1 $vogls_assert_eq(pos, 2);

        for (i = 0; i < 16; i = i + 1) begin
            in = i;
            #1 $vogls_assert_eq(pos, 
                i[0] ? 0 : (
                    i[1] ? 1 : (
                        i[2] ? 2 : (
                            i[3] ? 3 : 0
                        )
                    )
                )
            );
        end
    end
endmodule
