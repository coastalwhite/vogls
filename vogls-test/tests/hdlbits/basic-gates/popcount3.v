module top_module( 
    input [3:0] in,
    output [2:0] out_both,
    output [3:1] out_any,
    output [3:0] out_different );

    assign out_both = in[2:0] & in[3:1];
    assign out_any = in[3:1] | in[2:0];
    assign out_different = in ^ { in[0], in[3:1] };
endmodule

module tb();
    reg [3:0] in;
    wire [2:0] out_both;
    wire [3:1] out_any;
    wire [3:0] out_different;

    top_module i( in, out_both, out_any, out_different );

    initial begin
        in=4'b0000; #1 $vogls_assert_eq(out_both, 3'b000); $vogls_assert_eq(out_any, 3'b000); $vogls_assert_eq(out_different, 4'b0000);
        in=4'b0001; #1 $vogls_assert_eq(out_both, 3'b000); $vogls_assert_eq(out_any, 3'b001); $vogls_assert_eq(out_different, 4'b1001);
        in=4'b0010; #1 $vogls_assert_eq(out_both, 3'b000); $vogls_assert_eq(out_any, 3'b011); $vogls_assert_eq(out_different, 4'b0011);
        in=4'b0011; #1 $vogls_assert_eq(out_both, 3'b001); $vogls_assert_eq(out_any, 3'b011); $vogls_assert_eq(out_different, 4'b1010);
        in=4'b0100; #1 $vogls_assert_eq(out_both, 3'b000); $vogls_assert_eq(out_any, 3'b110); $vogls_assert_eq(out_different, 4'b0110);
        in=4'b0101; #1 $vogls_assert_eq(out_both, 3'b000); $vogls_assert_eq(out_any, 3'b111); $vogls_assert_eq(out_different, 4'b1111);
        in=4'b0110; #1 $vogls_assert_eq(out_both, 3'b010); $vogls_assert_eq(out_any, 3'b111); $vogls_assert_eq(out_different, 4'b0101);
        in=4'b0111; #1 $vogls_assert_eq(out_both, 3'b011); $vogls_assert_eq(out_any, 3'b111); $vogls_assert_eq(out_different, 4'b1100);
        in=4'b1000; #1 $vogls_assert_eq(out_both, 3'b000); $vogls_assert_eq(out_any, 3'b100); $vogls_assert_eq(out_different, 4'b1100);
        in=4'b1001; #1 $vogls_assert_eq(out_both, 3'b000); $vogls_assert_eq(out_any, 3'b101); $vogls_assert_eq(out_different, 4'b0101);
        in=4'b1010; #1 $vogls_assert_eq(out_both, 3'b000); $vogls_assert_eq(out_any, 3'b111); $vogls_assert_eq(out_different, 4'b1111);
        in=4'b1011; #1 $vogls_assert_eq(out_both, 3'b001); $vogls_assert_eq(out_any, 3'b111); $vogls_assert_eq(out_different, 4'b0110);
        in=4'b1100; #1 $vogls_assert_eq(out_both, 3'b100); $vogls_assert_eq(out_any, 3'b110); $vogls_assert_eq(out_different, 4'b1010);
        in=4'b1101; #1 $vogls_assert_eq(out_both, 3'b100); $vogls_assert_eq(out_any, 3'b111); $vogls_assert_eq(out_different, 4'b0011);
        in=4'b1110; #1 $vogls_assert_eq(out_both, 3'b110); $vogls_assert_eq(out_any, 3'b111); $vogls_assert_eq(out_different, 4'b1001);
        in=4'b1111; #1 $vogls_assert_eq(out_both, 3'b111); $vogls_assert_eq(out_any, 3'b111); $vogls_assert_eq(out_different, 4'b0000);
    end
endmodule
