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

// An input tied to a constant never changes, so the entries with an edge on it never match: a
// change of the data must not be taken for a rising edge of the clock tied to 1.
module tb();
    reg c, d;
    wire q;

    n_iq_ff (q, c, 1'b0, 1'b1, d, 1'b0);

    initial begin
        #0 c = 0; d = 0;
        #5 d = 1; #5 $vogls_assert_eq(q === 1'b1, 1'b0);
        c = 1;    #5 $vogls_assert_eq(q, 0);
        c = 0;    #5 $vogls_assert_eq(q, 0);
        d = 0;    #5 $vogls_assert_eq(q, 0);
        d = 1;    #5 $vogls_assert_eq(q, 0);
    end
endmodule
