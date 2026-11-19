// The writes of a task to an output with module path delays go through the delay, blocking and
// non-blocking alike.
`timescale 1ns/1ns
module cell_q(input a, output reg q, output reg r);
    task set_q;
        q = a;
    endtask
    task set_r;
        r <= a;
    endtask

    always @(a) begin
        set_q;
        set_r;
    end

    specify
        (a => q) = 3;
        (a => r) = 5;
    endspecify
endmodule

module top();
    reg a;
    wire q, r;
    cell_q u (a, q, r);

    initial begin
        a = 0;
        #10;
        $vogls_assert_eq(q, 1'b0);
        $vogls_assert_eq(r, 1'b0);

        a = 1;
        #2;
        $vogls_assert_eq(q, 1'b0);
        #2;
        $vogls_assert_eq(q, 1'b1);
        $vogls_assert_eq(r, 1'b0);
        #2;
        $vogls_assert_eq(r, 1'b1);
        $finish;
    end
endmodule
