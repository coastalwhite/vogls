module top_module(
    input a, 
    input b,
    output wire out_assign,
    output reg out_alwaysblock
);
    assign out_assign = a & b;
    always @(*) out_alwaysblock = a & b;
endmodule

module tb();
    reg a, b;
    wire out_assign, out_alwaysblock;

    top_module m(a, b, out_assign, out_alwaysblock);

    initial begin
        #1 a = 0; b = 0;
        #1 $vogls_assert_eq(out_assign, 0); $vogls_assert_eq(out_alwaysblock, 0);

        #1 a = 0; b = 1;
        #1 $vogls_assert_eq(out_assign, 0); $vogls_assert_eq(out_alwaysblock, 0);

        #1 a = 1; b = 1;
        #1 $vogls_assert_eq(out_assign, 1); $vogls_assert_eq(out_alwaysblock, 1);

        #1 a = 1; b = 0;
        #1 $vogls_assert_eq(out_assign, 0); $vogls_assert_eq(out_alwaysblock, 0);
    end
endmodule
