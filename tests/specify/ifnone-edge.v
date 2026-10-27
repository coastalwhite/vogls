// vogls: verify-stdout[sort-lines]
//
// An `ifnone` with an edge-sensitive path. The standard only allows a simple path after `ifnone`,
// but the gf180mcu cell library uses this shape.
`timescale 1ns/1ns
module gate(
    input a, b,
    output s
);
    assign s = a;

    specify
        if (b === 1'b0) (posedge a => (s : a)) = 1;
        ifnone          (posedge a => (s : a)) = 4;
        if (b === 1'b0) (negedge a => (s : a)) = 5;
        ifnone          (negedge a => (s : a)) = 6;
    endspecify
endmodule

module tb();
    reg a, b;
    wire s;

    gate g(a, b, s);

    // Skip the output settling at the start of the simulation.
    always @(s) if ($time >= 10) $display("[T=%0t] s = %b", $time, s);

    initial begin
        a = 0; b = 0;
        #10 a = 1;      // b = 0: the `if` paths
        #10 a = 0;
        #10 b = 1;
        #10 a = 1;      // b = 1: the `ifnone` paths
        #10 a = 0;
    end
endmodule
