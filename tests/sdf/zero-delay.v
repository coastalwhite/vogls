// vogls: verify-stdout
// vogls: annotate-sdf
// vogls: mode=four-value-logic
// Path delays that are zero after the SDF annotation, from an `INTERCONNECT`, an `IOPATH` and a
// specify block, pass their input on as if there were no delay. The delay of `u3` stays.
`timescale 1ns/1ps
module buffer(input a, output b);
    assign b = a;
    specify
        (a => b) = (2, 2);
    endspecify
endmodule
module edge_buffer(input clk, input d, output reg q);
    always @(posedge clk) q <= d;
    specify
        (posedge clk => (q : d)) = (0, 0);
    endspecify
endmodule
module top();
    reg in, clk;
    wire mid, out, late, q;

    buffer u1 (.a(in), .b(mid));
    buffer u2 (.a(mid), .b(out));
    buffer u3 (.a(out), .b(late));
    edge_buffer u4 (.clk(clk), .d(out), .q(q));

    task show;
        $display("[T=%0t] mid=%b out=%b late=%b q=%b", $realtime, mid, out, late, q);
    endtask
    initial begin
        clk = 0;
        in = 0;
        #1 show;
        #9 in = 1;
        #1 show;
        #2 show;
        #2 clk = 1;
        #1 show;
        #4 in = 0;
        #1 show;
        #2 show;
        #2 clk = 0;
        #5 clk = 1;
        #1 show;
        $finish;
    end
endmodule
