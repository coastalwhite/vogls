// vogls: mode=two-value-logic

// The process of a flop only wakes on the positive edge of its clock. That watch is armed from the
// start: the drives at `t=0` neither run the table, which would overwrite the initial state, nor
// get missed when the clock rises before the process would first run.
primitive dff_init1(q, clk, d);
    output q; reg q;
    input clk, d;
    initial q = 1;
table
    r 0 : ? : 0 ;
    r 1 : ? : 1 ;
    f ? : ? : - ;
    ? * : ? : - ;
endtable
endprimitive

module tb();
    reg clk_a, clk_b, d;
    wire qa, qb;

    dff_init1 (qa, clk_a, d);
    dff_init1 (qb, clk_b, d);

    initial begin
        #0 d = 0; clk_a = 0; clk_b = 1;
        #5 $vogls_assert_eq(qa, 1);
        $vogls_assert_eq(qb, 0);
        clk_a = 1; #5 $vogls_assert_eq(qa, 0);
    end
endmodule
