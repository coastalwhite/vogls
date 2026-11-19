// vogls: mode=four-value-logic
//
// `supply0` and `supply1` are always `0` and `1`, whatever drives them. `tri0` and `tri1` resolve
// their drivers like a `wire`, and their bits that are then `z` are `0` and `1`. A `uwire` is a
// `wire` with at most one driver per bit.
`timescale 1ns/1ns

module ports(input i0, input i1, output tri0 o_t0, output tri1 o_t1, output supply1 o_s1);
    assign o_t0 = i0;
    assign o_t0 = i1;
    assign o_t1 = i0;
    assign o_t1 = i1;
    assign o_s1 = i0;
endmodule

`default_nettype tri1
module implicit(input i0, input en, output o);
    // `n` is declared implicitly by the gate terminal, as a `tri1`.
    bufif1 (n, i0, en);
    assign o = n;
endmodule
`default_nettype wire

module tb();
    reg a, b, en;

    supply0 s0;
    supply1 s1;
    assign s0 = a;
    assign s0 = b;
    assign s1 = a;
    supply0 [3:0] s0_undriven;

    tri0 t0;
    tri1 t1;
    assign t0 = a;
    assign t0 = b;
    assign t1 = a;
    assign t1 = b;

    tri0 t0_single;
    tri1 t1_single;
    assign t0_single = a;
    assign t1_single = a;

    tri0 t0_undriven;
    tri1 [1:0] t1_undriven;

    // Only some bits are driven: the others are pulled.
    tri1 [3:0] t1_vec;
    assign t1_vec[1:0] = {a, b};
    assign t1_vec[1] = b;
    tri0 [3:0] t0_vec;
    assign t0_vec[2:1] = {a, b};

    uwire u;
    assign u = a;
    uwire [1:0] u_vec;
    assign u_vec[0] = a;
    assign u_vec[1] = b;

    wire p_t0, p_t1, p_s1;
    ports p(a, b, p_t0, p_t1, p_s1);

    wire implicit_o;
    implicit im(a, en, implicit_o);

    integer i;
    reg [3:0] values;
    initial begin
        values = 4'b01xz;
        for (i = 0; i < 32; i = i + 1) begin
            a = values[3 - i[3:2]];
            b = values[3 - i[1:0]];
            en = i[4];
            #1;

            $vogls_assert_eq(s0, 1'b0);
            $vogls_assert_eq(s1, 1'b1);
            $vogls_assert_eq(s0_undriven, 4'b0000);

            $vogls_assert_eq(t0, $vogls_resolve_wor($vogls_resolve_wire(a, b), 1'b0));
            $vogls_assert_eq(t1, $vogls_resolve_wand($vogls_resolve_wire(a, b), 1'b1));
            $vogls_assert_eq(t0_single, $vogls_resolve_wor(a, 1'b0));
            $vogls_assert_eq(t1_single, $vogls_resolve_wand(a, 1'b1));
            $vogls_assert_eq(t0_undriven, 1'b0);
            $vogls_assert_eq(t1_undriven, 2'b11);

            $vogls_assert_eq(t1_vec, {
                2'b11,
                $vogls_resolve_wand($vogls_resolve_wire(a, b), 1'b1),
                $vogls_resolve_wand(b, 1'b1)
            });
            $vogls_assert_eq(t0_vec, {
                1'b0,
                $vogls_resolve_wor(a, 1'b0),
                $vogls_resolve_wor(b, 1'b0),
                1'b0
            });

            $vogls_assert_eq(u, a);
            $vogls_assert_eq(u_vec, {b, a});

            $vogls_assert_eq(p_t0, $vogls_resolve_wor($vogls_resolve_wire(a, b), 1'b0));
            $vogls_assert_eq(p_t1, $vogls_resolve_wand($vogls_resolve_wire(a, b), 1'b1));
            $vogls_assert_eq(p_s1, 1'b1);

            // A disabled `bufif1` drives `z`, which is pulled. An enabled one drives `x` for an
            // input that is `z`.
            $vogls_assert_eq(implicit_o, en ? ~~a : 1'b1);
        end
    end
endmodule
