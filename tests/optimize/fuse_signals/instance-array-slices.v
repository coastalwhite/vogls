module inv(input [3:0] i, output [3:0] o);
    assign o = ~i;
endmodule

module top;
    reg [11:0] in;
    wire [11:0] out;

    inv u[2:0] (.i(in), .o(out));

    initial begin
        in = 12'h5A3;
        #1;
        $vogls_assert_eq(out, 12'hA5C);
        in = 12'h0F0;
        #1;
        $vogls_assert_eq(out, 12'hF0F);
    end
endmodule
