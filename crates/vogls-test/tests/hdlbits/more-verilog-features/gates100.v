module top_module (
    input [99:0] in,
    output out_and,
    output out_or,
    output out_xor
);
    assign out_and = &in;
    assign out_or = |in;
    assign out_xor = ^in;
endmodule

module tb();
    reg [99:0] in;
    wire out_and, out_or, out_xor;

    top_module m(in, out_and, out_or, out_xor);

    initial begin
        #1 in = 0;
        #1 $vogls_assert_eq(out_and, 0); $vogls_assert_eq(out_or, 0); $vogls_assert_eq(out_xor, 0); in = 1;
        #1 $vogls_assert_eq(out_and, 0); $vogls_assert_eq(out_or, 1); $vogls_assert_eq(out_xor, 1); in = { 1'b1, 98'b0, 1'b1 };
        #1 $vogls_assert_eq(out_and, 0); $vogls_assert_eq(out_or, 1); $vogls_assert_eq(out_xor, 0); in = ~100'b0;
        #1 $vogls_assert_eq(out_and, 1); $vogls_assert_eq(out_or, 1); $vogls_assert_eq(out_xor, 0); in = ~100'b0 ^ 1;
        #1 $vogls_assert_eq(out_and, 0); $vogls_assert_eq(out_or, 1); $vogls_assert_eq(out_xor, 1); 
    end
endmodule
