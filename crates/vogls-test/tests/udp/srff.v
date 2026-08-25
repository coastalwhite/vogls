primitive srff(q, s, r);
    output q; reg q;
    input s, r;
table
//  s   r     q     q+
    1   0  :  ?  :  1 ;
    f   0  :  1  :  - ;
    0   r  :  ?  :  0 ;
    0   f  :  0  :  - ;
    1   1  :  ?  :  0 ;
endtable
endprimitive

module tb();
    reg s, r;
    wire q;

    srff (q, s, r);

    initial begin
        #0 s = 1; r = 0;
        #5 $vogls_assert_eq(q, 1); s = 0;
        #5 $vogls_assert_eq(q, 1); r = 1;
        #5 $vogls_assert_eq(q, 0); r = 0;
        #5 $vogls_assert_eq(q, 0); s = 1;
        #5 $vogls_assert_eq(q, 1); r = 1;
        #5 $vogls_assert_eq(q, 0);
    end
endmodule