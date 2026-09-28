module ninput_gates(
    input a, b,
    output o_and, o_nand, o_or, o_nor, o_xor, o_xnor
);

    // and  (o_and,  a, b);
    nand (o_nand, a, b);
    // or   (o_or,   a, b);
    // nor  (o_nor,  a, b);
    // xor  (o_xor,  a, b);
    // xnor (o_xnor, a, b);
endmodule

module tb();
    reg a, b;
    wire o_and, o_nand, o_or, o_nor, o_xor, o_xnor;

    ninput_gates x(
        a, b,
        o_and, o_nand, o_or, o_nor, o_xor, o_xnor
    );

    initial begin
        a <= 0;
        b <= 0;
        #1
        // $vogls_assert_eq(o_and, 0);
        $vogls_assert_eq(o_nand, 1);
        // $vogls_assert_eq(o_or, 0);
        // $vogls_assert_eq(o_nor, 1);
        // $vogls_assert_eq(o_xor, 0);
        // $vogls_assert_eq(o_xnor, 1);

        // a <= 1;
        // b <= 0;
        // #1
        // $vogls_assert_eq(o_and, 0);
        // $vogls_assert_eq(o_nand, 1);
        // $vogls_assert_eq(o_or, 1);
        // $vogls_assert_eq(o_nor, 0);
        // $vogls_assert_eq(o_xor, 1);
        // $vogls_assert_eq(o_xnor, 0);
        //
        // a <= 1;
        // b <= 1;
        // #1
        // $vogls_assert_eq(o_and, 1);
        // $vogls_assert_eq(o_nand, 0);
        // $vogls_assert_eq(o_or, 1);
        // $vogls_assert_eq(o_nor, 0);
        // $vogls_assert_eq(o_xor, 0);
        // $vogls_assert_eq(o_xnor, 1);
        //
        // a <= 0;
        // b <= 1;
        // #1
        // $vogls_assert_eq(o_and, 0);
        // $vogls_assert_eq(o_nand, 1);
        // $vogls_assert_eq(o_or, 1);
        // $vogls_assert_eq(o_nor, 0);
        // $vogls_assert_eq(o_xor, 1);
        // $vogls_assert_eq(o_xnor, 0);
    end
endmodule