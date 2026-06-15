module top_module (
    input [7:0] in,
    output [31:0] out
);
    assign out = { {24{in[7]}}, in };
endmodule

module tb();
    reg [7:0] in;
    wire [31:0] out;

    top_module m(in, out);

    initial begin
        #1 in = 8'h00;
        #1 $vogls_assert_eq(out, 32'h0000_0000);

        #1 in = 8'hFF;
        #1 $vogls_assert_eq(out, 32'hFFFF_FFFF);

        #1 in = 8'hAB;
        #1 $vogls_assert_eq(out, 32'hFFFF_FFAB);

        #1 in = 8'h01;
        #1 $vogls_assert_eq(out, 32'h0000_0001);

        #1 in = 8'h7F;
        #1 $vogls_assert_eq(out, 32'h0000_007F);

        #1 in = 8'h80;
        #1 $vogls_assert_eq(out, 32'hFFFF_FF80);
    end
endmodule
