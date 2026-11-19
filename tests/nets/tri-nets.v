// vogls: mode=four-value-logic
//
// `tri`, `triand` and `trior` resolve their drivers like `wire`, `wand` and `wor`, also as ports
// and as `default_nettype`.
`timescale 1ns/1ns

module ansi_ports(input i0, input i1, output tri y_tri, output triand y_and, output trior y_or);
    assign y_tri = i0;
    assign y_tri = i1;
    assign y_and = i0;
    assign y_and = i1;
    assign y_or = i0;
    assign y_or = i1;
endmodule

module non_ansi_ports(i0, i1, y_and, y_or);
    input i0, i1;
    output y_and, y_or;
    triand y_and;
    trior y_or;
    assign y_and = i0;
    assign y_and = i1;
    assign y_or = i0;
    assign y_or = i1;
endmodule

// A `triand` and a `trior` port collapse with a `wand` and a `wor` net, as they are of the same
// net type.
module inout_ports(inout triand p_and, inout trior p_or, input i0);
    assign p_and = i0;
    assign p_or = i0;
endmodule

`default_nettype triand
module implicit_and(input i0, input i1, output o);
    buf (n, i0);
    buf (n, i1);
    assign o = n;
endmodule

`default_nettype trior
module implicit_or(input i0, input i1, output o);
    buf (n, i0);
    buf (n, i1);
    assign o = n;
endmodule
`default_nettype wire

module tb();
    reg a, b, c;

    wire ansi_tri, ansi_and, ansi_or;
    ansi_ports ansi(a, b, ansi_tri, ansi_and, ansi_or);
    wire non_ansi_and, non_ansi_or;
    non_ansi_ports non_ansi(a, b, non_ansi_and, non_ansi_or);

    wand w_and;
    wor w_or;
    assign w_and = b;
    assign w_and = c;
    assign w_or = b;
    assign w_or = c;
    inout_ports inouts(w_and, w_or, a);

    wire implicit_and_o, implicit_or_o;
    implicit_and i_and(a, b, implicit_and_o);
    implicit_or i_or(a, b, implicit_or_o);

    integer i;
    reg [3:0] values;
    initial begin
        values = 4'b01xz;
        for (i = 0; i < 64; i = i + 1) begin
            a = values[3 - i[5:4]];
            b = values[3 - i[3:2]];
            c = values[3 - i[1:0]];
            #1;

            $vogls_assert_eq(ansi_tri, $vogls_resolve_wire(a, b));
            $vogls_assert_eq(ansi_and, $vogls_resolve_wand(a, b));
            $vogls_assert_eq(ansi_or, $vogls_resolve_wor(a, b));
            $vogls_assert_eq(non_ansi_and, $vogls_resolve_wand(a, b));
            $vogls_assert_eq(non_ansi_or, $vogls_resolve_wor(a, b));

            $vogls_assert_eq(w_and, $vogls_resolve_wand($vogls_resolve_wand(b, c), a));
            $vogls_assert_eq(w_or, $vogls_resolve_wor($vogls_resolve_wor(b, c), a));

            // A `buf` drives `x` for an input that is `z`.
            $vogls_assert_eq(implicit_and_o, $vogls_resolve_wand(~~a, ~~b));
            $vogls_assert_eq(implicit_or_o, $vogls_resolve_wor(~~a, ~~b));
        end
    end
endmodule
