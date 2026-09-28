module add16 ( input [15:0] a, input[15:0] b, input cin, output [15:0] sum, output cout );
    assign { cout, sum } = { 1'b0, a } + { 1'b0, b } + cin;
endmodule

module top_module ( input [31:0] a, input [31:0] b, output [31:0] sum );
    wire lo_cout;
    wire [15:0] hi_c0_out, hi_c1_out;

    add16 lo    ( .a(a[15:0]),  .b(b[15:0]),  .cin(1'b0),    .sum(sum[15:0]),  .cout(lo_cout) );
    add16 hi_c0 ( .a(a[31:16]), .b(b[31:16]), .cin(1'b0),    .sum(hi_c0_out),  .cout()        );
    add16 hi_c1 ( .a(a[31:16]), .b(b[31:16]), .cin(1'b1),    .sum(hi_c1_out),  .cout()        );

    assign sum[31:16] = lo_cout ? hi_c1_out : hi_c0_out;
endmodule

module tb();
    reg [31:0] a, b;
    wire [31:0] sum;

    top_module m(a, b, sum);

    initial begin
        a = 0; b = 0;
        #1 $vogls_assert_eq(sum, 0);

        a = 1; b = 1;
        #1 $vogls_assert_eq(sum, 2);

        a = 32'hFFFF; b = 1;
        #1 $vogls_assert_eq(sum, 32'h1_0000);
        a = 32'hFFFF; b = 32'hFFFF;
        #1 $vogls_assert_eq(sum, 32'h1_FFFE);
        a = 32'hFFFF_FFFF; b = 32'hFFFF_FFFF;
        #1 $vogls_assert_eq(sum, 32'hFFFF_FFFE);
        a = 32'hABCD_EF01; b = 32'h9876_5432;
        #1 $vogls_assert_eq(sum, 32'h4444_4333);
    end
endmodule
