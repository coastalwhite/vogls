primitive n_iq_ff(Q, C, P, CK, D, N);
    output Q; reg Q;
    input C, P, CK, D, N;
table
//  C  P  CK  D  N  :  Q  :  Q
    0  0  n   ?  ?  :  ?  :  - ;
    ?  0  r   0  ?  :  ?  :  0 ;
    ?  0  p   0  ?  :  0  :  0 ;
    1  0  ?   ?  ?  :  ?  :  0 ;
    0  ?  r   1  ?  :  ?  :  1 ;
    0  ?  p   1  ?  :  1  :  1 ;
    0  1  ?   ?  ?  :  ?  :  1 ;
    ?  ?  ?   ?  *  :  ?  :  x ;
    0  0  ?   *  ?  :  ?  :  - ;
    0  n  ?   ?  ?  :  ?  :  - ;
    n  0  ?   ?  ?  :  ?  :  - ;
    0  p  ?   ?  ?  :  ?  :  - ;
endtable
endprimitive

// The clock is one bit of a vector: the process only watches that bit, for its positive edge in a
// two-valued design.
module tb();
    reg [3:0] bus;
    reg d;
    wire q;

    n_iq_ff (q, 1'b0, 1'b0, bus[2], d, 1'b0);

    initial begin
        #0 bus = 4'b0000; d = 1;
        #5 bus[2] = 1;     #5 $vogls_assert_eq(q, 1);
        d = 0;             #5 $vogls_assert_eq(q, 1);
        bus[1] = 1;        #5 $vogls_assert_eq(q, 1);
        bus[3] = 1;        #5 $vogls_assert_eq(q, 1);
        bus[2] = 0;        #5 $vogls_assert_eq(q, 1);
        bus[0] = 1;        #5 $vogls_assert_eq(q, 1);
        bus[2] = 1;        #5 $vogls_assert_eq(q, 0);
        bus = 4'b1011;     #5 $vogls_assert_eq(q, 0);
        d = 1;             #5;
        bus = 4'b0100;     #5 $vogls_assert_eq(q, 1);
    end
endmodule
