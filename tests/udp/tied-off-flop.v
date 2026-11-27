// The tie-offs of a standard cell flop leave a plain positive edge flop: the data input and the
// falling edge must not change the state.
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

module tb();
    reg clk, d;
    wire q;

    n_iq_ff (q, 1'b0, 1'b0, clk, d, 1'b0);

    initial begin
        #0 clk = 0; d = 1;
        #5 clk = 1; #5 $vogls_assert_eq(q, 1);
        d = 0;      #5 $vogls_assert_eq(q, 1);
        clk = 0;    #5 $vogls_assert_eq(q, 1);
        clk = 1;    #5 $vogls_assert_eq(q, 0);
        d = 1;      #5 $vogls_assert_eq(q, 0);
        clk = 0;    #5 $vogls_assert_eq(q, 0);
        clk = 1;    #5 $vogls_assert_eq(q, 1);
    end
endmodule
