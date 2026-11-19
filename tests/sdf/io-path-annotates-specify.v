// vogls: annotate-sdf
// vogls: mode=four-value-logic
//
// An SDF IOPATH annotates the module paths of a specify block between the same ports, instead of
// adding a second delay in front of them. Without an edge or condition it annotates every such
// path, also the conditional and edge-sensitive ones (IEEE Std 1364-2005, 16.2.1).
`timescale 1ns/1ns
module buffer(input a, output b);
    assign b = a;
    specify
        (a => b) = 1;
    endspecify
endmodule

module dff(input c, input e, input d, output reg q);
    always @(posedge c) if (e) q <= d;
    specify
        if (e) (posedge c => (q : d)) = 1;
    endspecify
endmodule

module top();
    reg a, c, e, d;
    wire b, q;

    buffer u_buf (a, b);
    dff u_dff (c, e, d, q);

    initial begin
        a = 0; c = 0; e = 1; d = 0;
        #10;
        $vogls_assert_eq(b, 1'b0);
        $vogls_assert_eq(q, 1'bx);

        a = 1;
        #2;
        $vogls_assert_eq(b, 1'b0);
        #2;
        $vogls_assert_eq(b, 1'b1);

        c = 1;
        #4;
        $vogls_assert_eq(q, 1'bx);
        #2;
        $vogls_assert_eq(q, 1'b0);
        $finish;
    end
endmodule
