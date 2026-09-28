module top_module( 
    input [2:0] a,
    input [2:0] b,
    output [2:0] out_or_bitwise,
    output out_or_logical,
    output [5:0] out_not
);
    assign out_or_bitwise = a | b;
    assign out_or_logical = a || b;
    assign out_not = { ~b, ~a };
endmodule

module tb();
    reg [2:0] a, b;
    wire [2:0] out_or_bitwise;
    wire out_or_logical;
    wire [5:0] out_not;

    top_module m(a, b, out_or_bitwise, out_or_logical, out_not);

    initial begin
        #1 a = 3'b000; b = 3'b000;
        #1
        $vogls_assert_eq(out_or_bitwise, 3'b000);
        $vogls_assert_eq(out_or_logical, 1'b0);
        $vogls_assert_eq(out_not, 6'b111_111);

        #1 a = 3'b111; b = 3'b111;
        #1
        $vogls_assert_eq(out_or_bitwise, 3'b111);
        $vogls_assert_eq(out_or_logical, 1'b1);
        $vogls_assert_eq(out_not, 6'b000_000);

        #1 a = 3'b001; b = 3'b011;
        #1
        $vogls_assert_eq(out_or_bitwise, 3'b011);
        $vogls_assert_eq(out_or_logical, 1'b1);
        $vogls_assert_eq(out_not, 6'b100_110);

        #1 a = 3'b110; b = 3'b000;
        #1
        $vogls_assert_eq(out_or_bitwise, 3'b110);
        $vogls_assert_eq(out_or_logical, 1'b1);
        $vogls_assert_eq(out_not, 6'b111_001);
    end
endmodule
