// vogls: annotate-sdf
// vogls: mode=four-value-logic
//
// An SDF COND annotates the module paths with the same condition, also when the SDF file writes
// the condition differently, and a CONDELSE annotates the `ifnone` paths. A COND without such a
// module path is ignored.
//
// Icarus Verilog does not match the conditions: it annotates every module path between the ports
// with each COND.
`timescale 1ns/1ns
module xor2(input a, input c, output y);
    assign y = a ^ c;
    specify
        if (!c) (a => y) = 1;
        ifnone (a => y) = 1;
        (c => y) = 1;
    endspecify
endmodule

module dffe(input clk, input e, input d, output reg q);
    always @(posedge clk) if (e) q <= d;
    specify
        if (e === 1'b1) (posedge clk => (q : d)) = 1;
    endspecify
endmodule

module top;
    reg a, c;
    wire y;
    xor2 u (a, c, y);

    reg clk, e, d;
    wire q;
    dffe f (clk, e, d, q);

    initial begin
        a = 0; c = 0;
        #10;
        $vogls_assert_eq(y, 1'b0);

        // COND c==1'b0
        a = 1;
        #1 $vogls_assert_eq(y, 1'b0);
        #2 $vogls_assert_eq(y, 1'b1);

        c = 1;
        #7;
        $vogls_assert_eq(y, 1'b0);

        // CONDELSE
        a = 0;
        #3 $vogls_assert_eq(y, 1'b0);
        #2 $vogls_assert_eq(y, 1'b1);

        // COND e of an edge-sensitive path
        clk = 0; e = 1; d = 1;
        #10 clk = 1;
        #5 $vogls_assert_eq(q, 1'bx);
        #2 $vogls_assert_eq(q, 1'b1);
        $finish;
    end
endmodule
