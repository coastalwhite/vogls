// vogls: annotate-sdf
// vogls: sdf-scope=tb.dut
//
// An SDF file that applies to a module instance below the top-level module, the way a testbench
// instantiates a design: its paths are relative to that instance, an empty INSTANCE is the instance
// itself, and a wildcard INSTANCE only matches the cells inside it.
`timescale 1ns/1ns
module buffer(input a, output b);
    assign b = a;
endmodule

module chip(input a, output b, output c);
    wire mid;
    buffer u1 (.a(a), .b(mid));
    buffer u2 (.a(mid), .b(b));
    assign c = a;
endmodule

module tb;
    reg a;
    wire b, c, d;
    chip dut (.a(a), .b(b), .c(c));
    // Outside of the scope, so the wildcard of the SDF file does not apply.
    buffer outside (.a(a), .b(d));

    initial begin
        a = 0;
        #20;
        $vogls_assert_eq(b, 1'b0);
        $vogls_assert_eq(c, 1'b0);
        $vogls_assert_eq(d, 1'b0);

        a = 1;
        #1;
        $vogls_assert_eq(d, 1'b1);
        #1;
        $vogls_assert_eq(c, 1'b0);
        #2;
        $vogls_assert_eq(c, 1'b1);
        $vogls_assert_eq(b, 1'b0);
        #2;
        // 2 for each buffer, 3 between them.
        $vogls_assert_eq(b, 1'b0);
        #2;
        $vogls_assert_eq(b, 1'b1);
        $finish;
    end
endmodule
