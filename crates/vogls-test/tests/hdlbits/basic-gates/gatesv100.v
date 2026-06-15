module top_module( 
    input [99:0] in,
    output [98:0] out_both,
    output [99:1] out_any,
    output [99:0] out_different );

    assign out_both = in[98:0] & in[99:1];
    assign out_any = in[99:1] | in[98:0];
    assign out_different = in ^ { in[0], in[99:1] };
endmodule

module tb();
    reg [99:0] in;
    wire [98:0] out_both;
    wire [99:1] out_any;
    wire [99:0] out_different;

    top_module i( in, out_both, out_any, out_different );

    initial begin
        in=100'b0;       #1 $vogls_assert_eq(out_both, 100'b0); $vogls_assert_eq(out_any, 100'b0); $vogls_assert_eq(out_different, 100'b0);
        in=100'b1;       #1 $vogls_assert_eq(out_both, 100'b0); $vogls_assert_eq(out_any, 100'b1); $vogls_assert_eq(out_different, {1'b1, 98'b0, 1'b1});
        in=100'b1 << 50; #1 $vogls_assert_eq(out_both, 100'b0); $vogls_assert_eq(out_any, 100'b11 << 49); $vogls_assert_eq(out_different, 98'b11 << 49);
    end
endmodule
