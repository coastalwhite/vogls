module top_module (
    input a, b, c, d, e,
    output [24:0] out
);
    assign out = ~{ {5{a}}, {5{b}}, {5{c}}, {5{d}}, {5{e}} } ^ {5{a, b, c, d, e}};
endmodule

module tb();
    reg a, b, c, d, e;
    wire [24:0] out;

    top_module m(a, b, c, d, e, out);

    initial begin
        #1 a = 0; b = 0; c = 0; d = 0; e = 0;
        #1 $vogls_assert_eq(out, 25'h1FF_FFFF);

        #1 a = 1; b = 1; c = 1; d = 1; e = 1;
        #1 $vogls_assert_eq(out, 25'h1FF_FFFF);

        #1 a = 1; b = 0; c = 1; d = 0; e = 1;
        #1 $vogls_assert_eq(out, 25'h1555555);
    end
endmodule
