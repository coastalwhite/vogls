`default_nettype none
module brev(
    input [31:0] in,
    output [31:0] out );
    assign out = { in[7:0], in[15:8], in[23:16], in[31:24] };
endmodule

module tb();
    reg [31:0] in;
    wire [31:0] out;

    brev m(in, out);

    initial begin
        #1 in = 32'haabb_ccdd;
        #1 $vogls_assert_eq(out, 32'hddcc_bbaa);

        #1 in = 32'h0000_0000;
        #1 $vogls_assert_eq(out, 32'h0000_0000);

        #1 in = 32'h0102_0304;
        #1 $vogls_assert_eq(out, 32'h0403_0201);
    end
endmodule
