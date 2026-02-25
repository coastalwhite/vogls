primitive rising_edge_detector(
    output z,
    input a
);
table
    (01) : ? : 1 ;
    ?    : ? : 0 ;
endtable
endprimitive

module tb();
    reg a;
    wire z;

    rising_edge_detector (z, a);

    initial begin
        #0 a = 0;
        #5 $vogls_assert_eq(z, 0); a = 1;
        #5 $vogls_assert_eq(z, 1); a = 1;
        #5 $vogls_assert_eq(z, 1); a = 0;
        #5 $vogls_assert_eq(z, 0); a = 0;
`ifndef __VOGLS__TWO_VALUE_LOGIC
        #5 $vogls_assert_eq(z, 0); a = 1'bx;
        #5 $vogls_assert_eq(z, 0); a = 1;
        #5 $vogls_assert_eq(z, 0); a = 1'bx;
        #5 $vogls_assert_eq(z, 0); a = 0;
`endif
        #5 $vogls_assert_eq(z, 0); a = 1;
        #5 $vogls_assert_eq(z, 1);
    end
endmodule
