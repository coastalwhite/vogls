module top_module( 
    input [255:0] in,
    input [7:0] sel,
    output out
); 
    assign out = in[sel];
endmodule

module tb();
    reg [255:0] in;
    reg [7:0] sel;
    wire out;

    top_module i(in, sel, out);

    initial begin
        in = 256'hABCD_0123_ABCD_2131__1238_1591_ABEF_1238__DE12_B00B_1413_1234__7598_9123_FF80_F012;
        sel=0;   #1 $vogls_assert_eq(out, 0);
        sel=1;   #1 $vogls_assert_eq(out, 1);
        sel=3;   #1 $vogls_assert_eq(out, 0);
        sel=42;  #1 $vogls_assert_eq(out, 0);
        sel=81;  #1 $vogls_assert_eq(out, 1);
        sel=255; #1 $vogls_assert_eq(out, 1);
        sel=121; #1 $vogls_assert_eq(out, 1);
        sel=119; #1 $vogls_assert_eq(out, 0);
    end
endmodule
