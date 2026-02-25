primitive xor_latch(
    output z,
    input a, b
);
table
    0 0 : 1 : 0 ;
    0 1 : 1 : 0 ;
    1 0 : 1 : 0 ;
    1 1 : 1 : 0 ;
    0 0 : ? : 0 ;
    0 1 : ? : 1 ;
    1 0 : ? : 1 ;
    1 1 : ? : 0 ;
endtable
endprimitive

module tb();
    reg a, b;
    wire z;

    xor_latch (z, a, b);

    initial begin
        #0 a = 0; b = 0;
        #5 $vogls_assert_eq(z, 0); a = 1;
        #5 $vogls_assert_eq(z, 1); b = 1;
        #5 $vogls_assert_eq(z, 0); a = 0;
        #5 $vogls_assert_eq(z, 1); a = 1; b = 0;
        #5 $vogls_assert_eq(z, 0);
    end
endmodule
