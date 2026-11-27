// vogls: mode=four-value-logic

// On a positive edge the state takes the value of the input after the edge, so the table is a copy
// of its input. The process only runs after a change of the input to `1` and back to `0` in the
// same time step, and must not take the `0` as the value after the edge.
primitive rise_follow(q, a);
    output q; reg q;
    input a;
table
    r    : ? : 1 ;
    (0x) : ? : x ;
    (x1) : ? : 1 ;
    n    : ? : - ;
endtable
endprimitive

module tb();
    reg a;
    wire q;

    rise_follow (q, a);

    initial begin
        #0 a = 0;
        #5 a = 1;    #5 $vogls_assert_eq(q, 1);
        a = 0;       #5 $vogls_assert_eq(q, 1);
        a = 1; a = 0; #5 $vogls_assert_eq(q === 1'b0, 1'b0);
        a = 1'bx;    #5 $vogls_assert_eq(q, 1'bx);
        a = 1;       #5 $vogls_assert_eq(q, 1);
    end
endmodule
