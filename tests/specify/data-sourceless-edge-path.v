// vogls: verify-stdout[sort-lines]
//
// An edge-sensitive path without a data source. The standard requires one, as in
// `(posedge a => (s : a))`.
`timescale 1ns/1ns
module gate(input a, output s);
    assign s = a;

    specify
        (posedge a => s) = 2;
        (negedge a => s) = 3;
    endspecify
endmodule

module tb();
    reg a;
    wire s;

    gate g(a, s);

    // Skip the output settling at the start of the simulation.
    always @(s) if ($time >= 10) $display("[T=%0t] s = %b", $time, s);

    initial begin
        a = 0;
        #10 a = 1;
        #10 a = 0;
    end
endmodule
