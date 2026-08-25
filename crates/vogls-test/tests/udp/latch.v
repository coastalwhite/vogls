primitive latch(q, clock, data);
    output q; reg q;
    input clock, data;
table
//    clock    data    q      q+
      0        1  :    ? :    1 ;
      0        0  :    ? :    0 ;
      1        ?  :    ? :    - ;   // - = no change
endtable
endprimitive

module tb();
    reg clock, data;
    wire q;

    latch (q, clock, data);

    initial begin
        #0 clock = 0; data = 0;
        #5 $vogls_assert_eq(q, 0); data = 1;
        #5 $vogls_assert_eq(q, 1); clock = 1;
        #5 $vogls_assert_eq(q, 1); data = 0;
        #5 $vogls_assert_eq(q, 1); data = 1;
        #5 $vogls_assert_eq(q, 1); clock = 0;
        #5 $vogls_assert_eq(q, 1); data = 0;
        #5 $vogls_assert_eq(q, 0); clock = 1;
        #5 $vogls_assert_eq(q, 0); data = 1;
        #5 $vogls_assert_eq(q, 0);
    end
endmodule
