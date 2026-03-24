module add1 ( input a, input b, input cin, output sum, output cout );
    assign { cout, sum } = { 1'b0, a } + { 1'b0, b } + cin;
endmodule

module add16 ( input [15:0] a, input[15:0] b, input cin, output [15:0] sum, output cout );
    wire couts [14:0];
    add1 i1 ( .a(a[ 0]), .b(b[ 0]), .cin(cin      ), .sum(sum[ 0]), .cout(couts[ 0]) );
    add1 i2 ( .a(a[ 1]), .b(b[ 1]), .cin(couts[ 0]), .sum(sum[ 1]), .cout(couts[ 1]) );
    add1 i3 ( .a(a[ 2]), .b(b[ 2]), .cin(couts[ 1]), .sum(sum[ 2]), .cout(couts[ 2]) );
    add1 i4 ( .a(a[ 3]), .b(b[ 3]), .cin(couts[ 2]), .sum(sum[ 3]), .cout(couts[ 3]) );
    add1 i5 ( .a(a[ 4]), .b(b[ 4]), .cin(couts[ 3]), .sum(sum[ 4]), .cout(couts[ 4]) );
    add1 i6 ( .a(a[ 5]), .b(b[ 5]), .cin(couts[ 4]), .sum(sum[ 5]), .cout(couts[ 5]) );
    add1 i7 ( .a(a[ 6]), .b(b[ 6]), .cin(couts[ 5]), .sum(sum[ 6]), .cout(couts[ 6]) );
    add1 i8 ( .a(a[ 7]), .b(b[ 7]), .cin(couts[ 6]), .sum(sum[ 7]), .cout(couts[ 7]) );
    add1 i9 ( .a(a[ 8]), .b(b[ 8]), .cin(couts[ 7]), .sum(sum[ 8]), .cout(couts[ 8]) );
    add1 i10( .a(a[ 9]), .b(b[ 9]), .cin(couts[ 8]), .sum(sum[ 9]), .cout(couts[ 9]) );
    add1 i11( .a(a[10]), .b(b[10]), .cin(couts[ 9]), .sum(sum[10]), .cout(couts[10]) );
    add1 i12( .a(a[11]), .b(b[11]), .cin(couts[10]), .sum(sum[11]), .cout(couts[11]) );
    add1 i13( .a(a[12]), .b(b[12]), .cin(couts[11]), .sum(sum[12]), .cout(couts[12]) );
    add1 i14( .a(a[13]), .b(b[13]), .cin(couts[12]), .sum(sum[13]), .cout(couts[13]) );
    add1 i15( .a(a[14]), .b(b[14]), .cin(couts[13]), .sum(sum[14]), .cout(couts[14]) );
    add1 i16( .a(a[15]), .b(b[15]), .cin(couts[14]), .sum(sum[15]), .cout(cout     ) );
endmodule

module top_module ( input [31:0] a, input [31:0] b, output [31:0] sum );
    wire lo_cout;
    add16 lo ( .a(a[15:0]),  .b(b[15:0]),  .cin(1'b0),    .sum(sum[15:0]),  .cout(lo_cout) );
    add16 hi ( .a(a[31:16]), .b(b[31:16]), .cin(lo_cout), .sum(sum[31:16]), .cout()        );
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
