// vogls: mode=two-value-logic

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

// In a two-valued design, an input can still be `x`: the state takes it as `0`. The flop is still
// a copy of its data on the positive edge, and a change of the data does not change the state.
module tb();
    reg clk, d, en;
    wire q;

    n_iq_ff (q, 1'b0, 1'b0, clk, en ? d : 1'bx, 1'b0);

    initial begin
        #0 clk = 0; d = 1; en = 1;
        #5 clk = 1; #5 $vogls_assert_eq(q, 1);
        en = 0;     #5 $vogls_assert_eq(q, 1);
        clk = 0;    #5 $vogls_assert_eq(q, 1);
        clk = 1;    #5 $vogls_assert_eq(q, 0);
        en = 1;     #5 $vogls_assert_eq(q, 0);
        clk = 0;    #5 $vogls_assert_eq(q, 0);
        clk = 1;    #5 $vogls_assert_eq(q, 1);
    end
endmodule
