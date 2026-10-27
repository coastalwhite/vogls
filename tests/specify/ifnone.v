// vogls: mode=four-value-logic
// vogls: verify-stdout[sort-lines]
`timescale 1ns/1ns
module gate(
    input i, c,
    output o_cond, o_alone, o_order, o_edge
);
    assign o_cond = i, o_alone = i, o_order = i, o_edge = i;

    specify
        // Applies when `c` is not 1, including when it is x.
        if (c)  (i => o_cond)  = 1;
        ifnone  (i => o_cond)  = 5;

        // Without other paths, `ifnone` is an unconditional path.
        ifnone  (i => o_alone) = 3;

        // The order of the paths does not matter.
        ifnone  (i => o_order) = 7;
        if (!c) (i => o_order) = 2;

        // Edge-sensitive paths also have a condition: falling edges use the `ifnone` delay.
        (posedge i => (o_edge : i)) = 1;
        ifnone (i => o_edge) = 4;
    endspecify
endmodule

module tb();
    reg i, c;
    wire o_cond, o_alone, o_order, o_edge;

    gate g(i, c, o_cond, o_alone, o_order, o_edge);

    always @(o_cond)  if ($time >= 10) $display("[T=%0t] o_cond = %b",  $time, o_cond);
    always @(o_alone) if ($time >= 10) $display("[T=%0t] o_alone = %b", $time, o_alone);
    always @(o_order) if ($time >= 10) $display("[T=%0t] o_order = %b", $time, o_order);
    always @(o_edge)  if ($time >= 10) $display("[T=%0t] o_edge = %b",  $time, o_edge);

    initial begin
        i = 0; c = 1;
        #10 i = 1;
        #10 i = 0;
        #10 c = 0;
        #10 i = 1;
        #10 c = 1'bx;
        #10 i = 0;
    end
endmodule
