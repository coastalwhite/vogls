module add1 ( input a, input b, input cin, output sum, output cout );
    assign { cout, sum } = { 1'b0, a } + { 1'b0, b } + cin;
endmodule

module add4 ( input [3:0] a, input[3:0] b, input cin, output [3:0] sum, output cout );
    wire couts [2:0];
    add1 i1 ( .a(a[ 0]), .b(b[ 0]), .cin(cin      ), .sum(sum[ 0]), .cout(couts[ 0]) );
    add1 i2 ( .a(a[ 1]), .b(b[ 1]), .cin(couts[ 0]), .sum(sum[ 1]), .cout(couts[ 1]) );
    add1 i3 ( .a(a[ 2]), .b(b[ 2]), .cin(couts[ 1]), .sum(sum[ 2]), .cout(couts[ 2]) );
    add1 i4 ( .a(a[ 3]), .b(b[ 3]), .cin(couts[ 2]), .sum(sum[ 3]), .cout(cout     ) );
endmodule

module tb();
    reg [3:0] a, b;
    reg cin;
    wire [3:0] sum;
    wire cout;

    add4 m(a, b, cin, sum, cout);

    initial begin
        a = 0; b = 0; cin = 0;
        #1 $vogls_assert_eq(sum, 0); $vogls_assert_eq(cout, 0);

        a = 1; b = 1; cin = 0;
        #1 $vogls_assert_eq(sum, 2); $vogls_assert_eq(cout, 0);

        a = 4'hF; b = 1; cin = 0;
        #1 $vogls_assert_eq(sum, 32'h0); $vogls_assert_eq(cout, 32'h1);
        a = 4'hF; b = 0; cin = 1;
        #1 $vogls_assert_eq(sum, 32'h0); $vogls_assert_eq(cout, 32'h1);
        a = 4'hA; b = 4'h3; cin = 1;
        #1 $vogls_assert_eq(sum, 32'hE); $vogls_assert_eq(cout, 32'h0);
    end
endmodule
