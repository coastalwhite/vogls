// vogls: mode=four-value-logic
//
// Nets with several drivers resolve them according to their net type: `wire` and `tri` with
// IEEE Std 1364-2005 table 4-2, `wand` and `triand` with table 4-3 and `wor` and `trior` with
// table 4-4.
`timescale 1ns/1ns

module ansi_ports(input [1:0] a, input [1:0] b, output wand [1:0] y_and, output wor [1:0] y_or);
    assign y_and = a;
    assign y_and = b;
    assign y_or = a;
    assign y_or = b;
endmodule

module non_ansi_ports(a, b, y_and, y_or);
    input a, b;
    output y_and, y_or;
    wand y_and;
    wor y_or;
    assign y_and = a;
    assign y_and = b;
    assign y_or = a;
    assign y_or = b;
endmodule

module tb();
    reg a, b, c;
    reg en_a, en_b;

    wire w;
    tri t;
    wand wa;
    triand ta;
    wor wo;
    trior to;

    assign w = a;
    assign w = b;
    assign t = a;
    assign t = b;
    assign wa = a;
    assign wa = b;
    assign ta = a;
    assign ta = b;
    assign wo = a;
    assign wo = b;
    assign to = a;
    assign to = b;

    // Three drivers.
    wand wa3;
    wor wo3;
    assign wa3 = a;
    assign wa3 = b;
    assign wa3 = c;
    assign wo3 = a;
    assign wo3 = b;
    assign wo3 = c;

    // Gates as drivers, which drive `x` for an input that is `z`, and tri-state drivers that drive
    // `z` when disabled.
    wand wa_gates;
    wor wo_gates;
    and (wa_gates, a, c);
    buf (wa_gates, b);
    or (wo_gates, a, c);
    buf (wo_gates, b);

    wand wa_tri;
    wor wo_tri;
    bufif1 (wa_tri, a, en_a);
    bufif1 (wa_tri, b, en_b);
    bufif1 (wo_tri, a, en_a);
    bufif1 (wo_tri, b, en_b);

    // Drivers that only cover some bits: every bit is resolved with the drivers that cover it.
    wand [3:0] wa_vec;
    wor [3:0] wo_vec;
    assign wa_vec[1:0] = {a, b};
    assign wa_vec[2:1] = {b, c};
    assign wa_vec = {c, a, a, c};
    assign wo_vec[1:0] = {a, b};
    assign wo_vec[2:1] = {b, c};
    assign wo_vec = {c, a, a, c};

    wire [1:0] ansi_and, ansi_or;
    ansi_ports ansi({a, b}, {b, c}, ansi_and, ansi_or);
    wire non_ansi_and, non_ansi_or;
    non_ansi_ports non_ansi(a, c, non_ansi_and, non_ansi_or);

    integer i;
    reg [3:0] values;
    reg tri_a, tri_b;
    initial begin
        values = 4'b01xz;
        for (i = 0; i < 64; i = i + 1) begin
            a = values[3 - i[5:4]];
            b = values[3 - i[3:2]];
            c = values[3 - i[1:0]];
            en_a = i[0];
            en_b = i[1];
            #1;

            $vogls_assert_eq(w, $vogls_resolve_wire(a, b));
            $vogls_assert_eq(t, $vogls_resolve_wire(a, b));
            $vogls_assert_eq(wa, $vogls_resolve_wand(a, b));
            $vogls_assert_eq(ta, $vogls_resolve_wand(a, b));
            $vogls_assert_eq(wo, $vogls_resolve_wor(a, b));
            $vogls_assert_eq(to, $vogls_resolve_wor(a, b));

            $vogls_assert_eq(wa3, $vogls_resolve_wand($vogls_resolve_wand(a, b), c));
            $vogls_assert_eq(wo3, $vogls_resolve_wor($vogls_resolve_wor(a, b), c));

            $vogls_assert_eq(wa_gates, $vogls_resolve_wand(a & c, ~~b));
            $vogls_assert_eq(wo_gates, $vogls_resolve_wor(a | c, ~~b));

            tri_a = en_a ? ~~a : 1'bz;
            tri_b = en_b ? ~~b : 1'bz;
            $vogls_assert_eq(wa_tri, $vogls_resolve_wand(tri_a, tri_b));
            $vogls_assert_eq(wo_tri, $vogls_resolve_wor(tri_a, tri_b));

            $vogls_assert_eq(wa_vec, {
                c,
                $vogls_resolve_wand(b, a),
                $vogls_resolve_wand($vogls_resolve_wand(a, c), a),
                $vogls_resolve_wand(b, c)
            });
            $vogls_assert_eq(wo_vec, {
                c,
                $vogls_resolve_wor(b, a),
                $vogls_resolve_wor($vogls_resolve_wor(a, c), a),
                $vogls_resolve_wor(b, c)
            });

            $vogls_assert_eq(ansi_and, $vogls_resolve_wand({a, b}, {b, c}));
            $vogls_assert_eq(ansi_or, $vogls_resolve_wor({a, b}, {b, c}));
            $vogls_assert_eq(non_ansi_and, $vogls_resolve_wand(a, c));
            $vogls_assert_eq(non_ansi_or, $vogls_resolve_wor(a, c));
        end
    end
endmodule
