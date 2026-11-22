// vogls: mode=two-value-logic
// vogls: verify-stdout
//
// In two-valued logic, a closure of `tran`s or of `wire`s collapsed by `inout` ports resolves the
// drivers of its members to their `|`, so the bits that a member does not drive are `0`, which
// leaves the others as they are. A closure of `wand`s still resolves only the driven bits, as `0`
// is not the identity of `&`.
`timescale 1ns/1ns
module pass(inout [1:0] p);
endmodule
module wand_port(inout wand [1:0] p, input b);
    assign p[1] = b;
endmodule
module top;
    reg a = 0, b = 0;

    // `x[1]` and `y[0]` have no drivers.
    wire [1:0] x, y;
    assign x[0] = a;
    assign y[1] = b;
    tran t0 (x[0], y[0]);
    tran t1 (x[1], y[1]);

    // Contention through a `tran`.
    wire c, d;
    assign c = a;
    assign d = b;
    tran t2 (c, d);

    // Collapsed by an `inout` port, `q[1]` has no drivers.
    wire [1:0] q;
    assign q[0] = a;
    pass u (.p(q));

    // `wand`s collapsed by an `inout` port, where each side drives one bit: the bit that `m`
    // does not drive is not resolved as `0`.
    wand [1:0] m;
    assign m[0] = a;
    wand_port u0 (.p(m), .b(b));

    initial begin
        #1 $display("a=%b b=%b x=%b y=%b c=%b d=%b q=%b m=%b", a, b, x, y, c, d, q, m);
        a = 1;
        #1 $display("a=%b b=%b x=%b y=%b c=%b d=%b q=%b m=%b", a, b, x, y, c, d, q, m);
        b = 1;
        #1 $display("a=%b b=%b x=%b y=%b c=%b d=%b q=%b m=%b", a, b, x, y, c, d, q, m);
        a = 0;
        #1 $display("a=%b b=%b x=%b y=%b c=%b d=%b q=%b m=%b", a, b, x, y, c, d, q, m);
    end
endmodule
