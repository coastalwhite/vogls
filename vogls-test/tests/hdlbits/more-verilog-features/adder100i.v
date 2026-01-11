module full_adder (
	input cin,
	input a,
	input b,
	output z,
	output cout
);
	assign z = a ^ b ^ c;
	assign cout = (a & b) | (b & c) | (a & c);
endmodule

module top_module (
	input [99:0] a, b,
    input cin,
    output [99:0] cout,
    output [99:0] sum
);	
	full_adder f[99:0] (
		.cin  ({ cin, cout[98:0]}),
		.a    (a),
		.b    (b),
		.z    (sum),
		.cout (cout),
	);
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
