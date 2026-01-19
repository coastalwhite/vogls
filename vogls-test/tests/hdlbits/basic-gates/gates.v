module top_module( 
    input a, b,
    output out_and,
    output out_or,
    output out_xor,
    output out_nand,
    output out_nor,
    output out_xnor,
    output out_anotb
);
    and(out_and, a, b);
    or(out_or, a, b);
    xor(out_xor, a, b);
    nand(out_nand, a, b);
    nor(out_nor, a, b);
    xnor(out_xnor, a, b);
    and(out_anotb, a, ~b);
endmodule

module tb();
    reg a, b;
    wire out_and, out_or, out_xor, out_nand, out_nor, out_xnor, out_anotb;

    top_module i(a, b, out_and, out_or, out_xor, out_nand, out_nor, out_xnor, out_anotb);

    initial begin
        a = 0; b = 0;
        #1
        $vogls_assert_eq(out_and, 0);
        $vogls_assert_eq(out_or, 0);
        $vogls_assert_eq(out_xor, 0);
        $vogls_assert_eq(out_nand, 1);
        $vogls_assert_eq(out_nor, 1);
        $vogls_assert_eq(out_xnor, 1);
        $vogls_assert_eq(out_anotb, 0);

        a = 0; b = 1;
        #1
        $vogls_assert_eq(out_and, 0);
        $vogls_assert_eq(out_or, 1);
        $vogls_assert_eq(out_xor, 1);
        $vogls_assert_eq(out_nand, 1);
        $vogls_assert_eq(out_nor, 0);
        $vogls_assert_eq(out_xnor, 0);
        $vogls_assert_eq(out_anotb, 0);

        a = 1; b = 0;
        #1
        $vogls_assert_eq(out_and, 0);
        $vogls_assert_eq(out_or, 1);
        $vogls_assert_eq(out_xor, 1);
        $vogls_assert_eq(out_nand, 1);
        $vogls_assert_eq(out_nor, 0);
        $vogls_assert_eq(out_xnor, 0);
        $vogls_assert_eq(out_anotb, 1);

        a = 1; b = 1;
        #1
        $vogls_assert_eq(out_and, 1);
        $vogls_assert_eq(out_or, 1);
        $vogls_assert_eq(out_xor, 0);
        $vogls_assert_eq(out_nand, 0);
        $vogls_assert_eq(out_nor, 0);
        $vogls_assert_eq(out_xnor, 1);
        $vogls_assert_eq(out_anotb, 0);
    end
endmodule
