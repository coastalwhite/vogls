// vogls: mode=four-value-logic
//
// Implicit nets take their type from `default_nettype`.
`timescale 1ns/1ns
`default_nettype wor

module tb();
    reg a, b;

    // `implicit` is declared implicitly by the gate terminal, as a `wor`. A `buf` drives `x` for
    // an input that is `z`.
    buf (implicit, a);
    buf (implicit, b);

    integer i;
    reg [3:0] values;
    initial begin
        values = 4'b01xz;
        for (i = 0; i < 16; i = i + 1) begin
            a = values[3 - i[3:2]];
            b = values[3 - i[1:0]];
            #1 $vogls_assert_eq(implicit, $vogls_resolve_wor(~~a, ~~b));
        end
    end
endmodule

`default_nettype wire
