// vogls: mode=two-value-logic
// vogls: verify-stdout
//
// In two-valued logic, the drivers of a `wire` or `tri` resolve to their `|`: as in four-valued
// logic, except for contention, which is `1` instead of `x`. A bit without drivers is `0`.
`timescale 1ns/1ns
module top;
    reg a = 0, b = 0;
    wire w;
    tri t;
    wire [3:0] v;
    assign w = a;
    assign w = b;
    assign t = a;
    assign t = b;
    // Only bits 0 and 2 have drivers, and bit 0 has two.
    assign v[0] = a;
    assign v[0] = b;
    assign v[2] = b;

    initial begin
        #1 $display("a=%b b=%b w=%b t=%b v=%b", a, b, w, t, v);
        a = 1;
        #1 $display("a=%b b=%b w=%b t=%b v=%b", a, b, w, t, v);
        b = 1;
        #1 $display("a=%b b=%b w=%b t=%b v=%b", a, b, w, t, v);
        a = 0;
        #1 $display("a=%b b=%b w=%b t=%b v=%b", a, b, w, t, v);
    end
endmodule
