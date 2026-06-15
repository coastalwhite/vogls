module top_module( 
    input x3,
    input x2,
    input x1,  // three inputs
    output f   // one output
);
    assign f = 
        (~x3 & x2     ) |
        (x3 & ~x2 & x1) |
        (x3 &  x2 & x1);

endmodule

module tb();
    reg x3, x2, x1;
    wire f;

    top_module i(
        .x1(x1), .x2(x2), .x3(x3),
        .f(f)
    );

    initial begin
        x3=0;x2=0;x1=0; #1 $vogls_assert_eq(f, 0);
        x3=0;x2=0;x1=1; #1 $vogls_assert_eq(f, 0);
        x3=0;x2=1;x1=0; #1 $vogls_assert_eq(f, 1);
        x3=0;x2=1;x1=1; #1 $vogls_assert_eq(f, 1);
        x3=1;x2=0;x1=0; #1 $vogls_assert_eq(f, 0);
        x3=1;x2=0;x1=1; #1 $vogls_assert_eq(f, 1);
        x3=1;x2=1;x1=0; #1 $vogls_assert_eq(f, 0);
        x3=1;x2=1;x1=1; #1 $vogls_assert_eq(f, 1);
    end
endmodule
