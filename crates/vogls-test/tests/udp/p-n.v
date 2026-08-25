// vogls: mode=four-value-logic
primitive pdet(z, a);
    output z; reg z;
    input a;
table
    p : ? : 1 ;
    ? : ? : 0 ;
endtable
endprimitive

primitive ndet(z, a);
    output z; reg z;
    input a;
table
    n : ? : 1 ;
    ? : ? : 0 ;
endtable
endprimitive

module tb();
    reg a;
    wire zp, zn;

    pdet (zp, a);
    ndet (zn, a);

    initial begin
        #0 a = 1'b0;
        #5 a = 1'b1;   // (01): p
        #5 $vogls_assert_eq(zp, 1); $vogls_assert_eq(zn, 0);
           a = 1'b0;   // (10): n
        #5 $vogls_assert_eq(zp, 0); $vogls_assert_eq(zn, 1);
           a = 1'bx;   // (0x): p
        #5 $vogls_assert_eq(zp, 1); $vogls_assert_eq(zn, 0);
           a = 1'b1;   // (x1): p
        #5 $vogls_assert_eq(zp, 1); $vogls_assert_eq(zn, 0);
           a = 1'bx;   // (1x): n
        #5 $vogls_assert_eq(zp, 0); $vogls_assert_eq(zn, 1);
           a = 1'b0;   // (x0): n
        #5 $vogls_assert_eq(zp, 0); $vogls_assert_eq(zn, 1);
    end
endmodule
