module top_module (
    input [7:0] in,
    output [7:0] out
);
    integer i;
    always @ (in) begin
        for (i = 0; i < 8; i = i + 1) out[i] = in[7-i];
    end
endmodule

module tb();
    reg [7:0] in;
    wire [7:0] out;

    top_module m(in, out);

    initial begin
        #1 in = 8'h00;
        #1 $vogls_assert_eq(out, 8'h00);

        #1 in = 8'hFF;
        #1 $vogls_assert_eq(out, 8'hFF);

        #1 in = 8'hAB;
        #1 $vogls_assert_eq(out, 8'hD5);

        #1 in = 8'h01;
        #1 $vogls_assert_eq(out, 8'h80);
    end
endmodule
