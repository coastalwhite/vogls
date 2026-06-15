// vogls: fail
module add1 ( input a, input b, input cin, output sum, output cout );
    assign { cout, sum } = { 1'b0, a } + { 1'b0, b } + cin;
endmodule

module add4 ( input [15:0] a, input[15:0] b, input cin, output [15:0] sum, output cout );
    wire couts [4:0];
    genvar i;
    generate
    for (i = 0; i < 4; i = i + 1) begin
        add1 i(
            .a(a[i]),
            .b(b[i]),
            .cin(couts[i]),
            .sum(sum[i]),
            .cout(couts[i+1])
        );
    end
    endgenerate

    assign couts[0] = 1'b0;
    assign cout = couts[4];
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
    end
endmodule
