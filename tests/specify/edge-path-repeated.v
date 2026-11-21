// vogls: verify-stdout
//
// An edge-sensitive path delays every output change, not only the first. The output only changes
// on the edge of the input, so the input's value when the output last changed is not its value
// before its last edge.
`timescale 1ns/1ps
module dff_p(input D, input CLK, output Q);
    reg q;
    always @(posedge CLK) q <= D;
    buf (Q, q);

    specify
        (posedge CLK => (Q : D)) = (1.5, 2.5);
    endspecify
endmodule

module dff_n(input D, input CLK, output Q);
    reg q;
    always @(negedge CLK) q <= D;
    buf (Q, q);

    specify
        (negedge CLK => (Q : D)) = (3, 4);
    endspecify
endmodule

module tb();
    reg clk = 0, d = 0;
    wire qp, qn;

    dff_p p(d, clk, qp);
    dff_n n(d, clk, qn);

    always #10 clk = ~clk;

    // Skip the outputs settling at the start of the simulation.
    always @(qp) if ($time >= 10) $display("[T=%0t] qp = %b", $time, qp);
    always @(qn) if ($time >= 10) $display("[T=%0t] qn = %b", $time, qn);

    initial begin
        #5 d = 1;
        #40 d = 0;
        #40 d = 1;
        #40 $finish;
    end
endmodule
