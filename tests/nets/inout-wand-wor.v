// vogls: mode=four-value-logic
//
// An `inout` port collapses with the nets it is connected to into one net: the drivers of all of
// them are resolved together with their common net type.
`timescale 1ns/1ns
module and_port(inout wand p, input i0, input i1);
    assign p = i0;
    assign p = i1;
endmodule

module or_port(inout wor p, input i0, input i1);
    assign p = i0;
    assign p = i1;
endmodule

module tb();
    reg a, b, c, d;

    wand x;
    assign x = c;
    assign x = d;
    and_port inst_and(x, a, b);

    wor y;
    assign y = c;
    assign y = d;
    or_port inst_or(y, a, b);

    integer i;
    reg [3:0] values;
    reg expected;
    initial begin
        values = 4'b01xz;
        for (i = 0; i < 256; i = i + 1) begin
            a = values[3 - i[7:6]];
            b = values[3 - i[5:4]];
            c = values[3 - i[3:2]];
            d = values[3 - i[1:0]];
            #1;

            expected = $vogls_resolve_wand($vogls_resolve_wand(a, b), $vogls_resolve_wand(c, d));
            $vogls_assert_eq(x, expected);
            $vogls_assert_eq(inst_and.p, expected);

            expected = $vogls_resolve_wor($vogls_resolve_wor(a, b), $vogls_resolve_wor(c, d));
            $vogls_assert_eq(y, expected);
            $vogls_assert_eq(inst_or.p, expected);
        end
    end
endmodule
