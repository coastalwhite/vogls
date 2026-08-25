primitive multiplexer(mux, control, dataA, dataB);
    output mux;
    input control, dataA, dataB;
table
//     control   dataA    dataB      mux
       0         1        0  :       1 ;
       0         1        1  :       1 ;
       0         1        x  :       1 ;
       0         0        0  :       0 ;
       0         0        1  :       0 ;
       0         0        x  :       0 ;
       1         0        1  :       1 ;
       1         1        1  :       1 ;
       1         x        1  :       1 ;
       1         0        0  :       0 ;
       1         1        0  :       0 ;
       1         x        0  :       0 ;
       x         0        0  :       0 ;
       x         1        1  :       1 ;
endtable
endprimitive

primitive multiplexer_short(mux, control, dataA, dataB);
    output mux;
    input control, dataA, dataB;
table
//     control      dataA     dataB   mux
       0            1         ?  :    1 ;
       0            0         ?  :    0 ;
       1            ?         1  :    1 ;
       1            ?         0  :    0 ;
       x            0         0  :    0 ;
       x            1         1  :    1 ;
endtable
endprimitive

module tb();
    reg control, dataA, dataB;
    wire m, ms;

    multiplexer (m, control, dataA, dataB);
    multiplexer_short (ms, control, dataA, dataB);

    initial begin
        #0 control = 0; dataA = 1; dataB = 0;
        #5 $vogls_assert_eq(m, 1); $vogls_assert_eq(ms, 1);
           control = 0; dataA = 0; dataB = 1;
        #5 $vogls_assert_eq(m, 0); $vogls_assert_eq(ms, 0);
           control = 1; dataA = 0; dataB = 1;
        #5 $vogls_assert_eq(m, 1); $vogls_assert_eq(ms, 1);
           control = 1; dataA = 1; dataB = 0;
        #5 $vogls_assert_eq(m, 0); $vogls_assert_eq(ms, 0);
`ifndef __VOGLS__TWO_VALUE_LOGIC
           control = 1'bx; dataA = 1; dataB = 1;
        #5 $vogls_assert_eq(m, 1); $vogls_assert_eq(ms, 1);
           control = 1'bx; dataA = 0; dataB = 0;
        #5 $vogls_assert_eq(m, 0); $vogls_assert_eq(ms, 0);
           control = 0; dataA = 1'bx; dataB = 1'bx;
        #5 $vogls_assert_eq(m, 1'bx); $vogls_assert_eq(ms, 1'bx);
`endif
    end
endmodule
