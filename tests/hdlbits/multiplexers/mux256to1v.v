module top_module( 
    input [1023:0] in,
    input [7:0] sel,
    output [3:0] out
);
    assign out = in[{sel, 2'b00} +: 4];
endmodule

module tb();
    reg [1023:0] in;
    reg [7:0] sel;
    wire [3:0] out;

    top_module i(in, sel, out);

    initial begin
        in = {4{256'hABCD_0123_ABCD_2131__1238_1591_ABEF_1238__DE12_B00B_1413_1234__7598_9123_FF80_F012}};
        sel=0;   #1 $vogls_assert_eq(out, 4'h2);
        sel=1;   #1 $vogls_assert_eq(out, 4'h1);
        sel=3;   #1 $vogls_assert_eq(out, 4'hF);
        sel=42;  #1 $vogls_assert_eq(out, 4'h5);
        sel=81;  #1 $vogls_assert_eq(out, 4'h3);
        sel=255; #1 $vogls_assert_eq(out, 4'hA);
        sel=121; #1 $vogls_assert_eq(out, 4'h2);
        sel=119; #1 $vogls_assert_eq(out, 4'hA);
    end
endmodule
