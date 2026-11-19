// vogls: mode=four-value-logic
//
// The nets that a `tran` connects resolve their own drivers with their own net type, and are then
// resolved with each other as `wire`s, as in Icarus Verilog.
`timescale 1ns/1ns
module tb();
    reg a, b, c, d;

    wand x;
    wor y;
    assign x = a;
    assign x = b;
    assign y = c;
    assign y = d;
    tran (x, y);

    // The `tran` comes before the drivers.
    wand p;
    wand q;
    tran (p, q);
    assign p = a;
    assign p = b;
    assign q = c;
    assign q = d;

    // Only some bits are connected.
    wor [1:0] u;
    wand [1:0] v;
    assign u = {a, b};
    assign u = {c, d};
    assign v = {b, a};
    assign v = {d, c};
    tran (u[0], v[1]);

    integer i;
    reg [3:0] values;
    reg joined;
    initial begin
        values = 4'b01xz;
        for (i = 0; i < 256; i = i + 1) begin
            a = values[3 - i[7:6]];
            b = values[3 - i[5:4]];
            c = values[3 - i[3:2]];
            d = values[3 - i[1:0]];
            #1;

            joined = $vogls_resolve_wire($vogls_resolve_wand(a, b), $vogls_resolve_wor(c, d));
            $vogls_assert_eq(x, joined);
            $vogls_assert_eq(y, joined);

            joined = $vogls_resolve_wire($vogls_resolve_wand(a, b), $vogls_resolve_wand(c, d));
            $vogls_assert_eq(p, joined);
            $vogls_assert_eq(q, joined);

            joined = $vogls_resolve_wire($vogls_resolve_wor(b, d), $vogls_resolve_wand(b, d));
            $vogls_assert_eq(u, {$vogls_resolve_wor(a, c), joined});
            $vogls_assert_eq(v, {joined, $vogls_resolve_wand(a, c)});
        end
    end
endmodule
