module top_module (
    input [254:0] in,
	output [7:0] out
);
	integer i = 0;
	always @(*) begin
		out = 0;
		for (i = 0; i < 255; i = i + 1) out = out + { 7'b0, in[i] };
	end
endmodule

module tb();
    reg [254:0] in;
    wire [7:0] out;

    top_module m(in, out);

    initial begin
        #1 in = 0;
        #1 $vogls_assert_eq(out, 0); in = 1;
        #1 $vogls_assert_eq(out, 1); in = 100'h1_1234_5678_aaaa_bbbb_cccc_dddd;
        #1 $vogls_assert_eq(out, 54); in = 255'hFFFFFFFF_FFFFFFFF_FFFFFFFF_FFFFFFFF_FFFFFFFF_FFFFFFFF_FFFFFFFF_FFFFFFFF;
        #1 $vogls_assert_eq(out, 255);
    end
endmodule
