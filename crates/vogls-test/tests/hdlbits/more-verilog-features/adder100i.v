`define WIDTH 99
module full_adder (
	input cin,
	input a,
	input b,
	output z,
	output cout
);
	assign z = a ^ b ^ cin;
	assign cout = (a & b) | (b & cin) | (a & cin);
endmodule

module top_module (
	input [`WIDTH:0] a, b,
    input cin,
    output [`WIDTH:0] cout,
    output [`WIDTH:0] sum
);	
	full_adder f[`WIDTH:0] (
		.cin  ({ cout[`WIDTH - 1:0], cin }),
		.a    (a),
		.b    (b),
		.z    (sum),
		.cout (cout)
	);
endmodule

module tb();
    reg [`WIDTH:0] a, b;
    reg cin;
    wire [`WIDTH:0] cout, sum;

    top_module m(
        .a(a),
        .b(b),
        .cin(cin),
        .cout(cout),
        .sum(sum)
    );

    initial begin
        a = 0; b = 0; cin = 0;
        #1 $vogls_assert_eq(cout, 0); $vogls_assert_eq(sum, 0); a = 1'b1;
        #1 $vogls_assert_eq(cout, 0); $vogls_assert_eq(sum, 1); b = 1'b1;
        #1 $vogls_assert_eq(cout, 1); $vogls_assert_eq(sum, 2); a = 4'b1001;
        #1 $vogls_assert_eq(cout, 1); $vogls_assert_eq(sum, 4'b1010);
    end
endmodule
