// vogls: mode=four-value-logic

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

// The inputs are bits of one vector. A change of another bit of that vector is no change of an
// input: the table must not be evaluated, as no entry would match and the state would become `x`.
module tb();
    reg [3:0] v;
    wire q;

    // The clear, the clock and the data, with the clear making more than one input change the
    // state.
    n_iq_ff (q, v[0], 1'b0, v[1], v[2], 1'b0);

    initial begin
        #0 v = 4'b0100;
        #5 v[1] = 1;    #5 $vogls_assert_eq(q, 1);
        v[3] = 1;       #5 $vogls_assert_eq(q, 1);
        v[3] = 0;       #5 $vogls_assert_eq(q, 1);
        v[1] = 0;       #5 $vogls_assert_eq(q, 1);
        v[0] = 1;       #5 $vogls_assert_eq(q, 0);
        v[0] = 0;       #5 $vogls_assert_eq(q, 0);
        v[3] = 1;       #5 $vogls_assert_eq(q, 0);
    end
endmodule
