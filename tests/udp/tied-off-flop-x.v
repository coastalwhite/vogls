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

// The transitions of the clock from and to `x` merge the data with the state.
module tb();
    reg clk, d;
    wire q;

    n_iq_ff (q, 1'b0, 1'b0, clk, d, 1'b0);

    initial begin
        #0 clk = 0; d = 1;
        #5 $vogls_assert_eq(q, 1'bx);
        clk = 1;    #5 $vogls_assert_eq(q, 1);
        clk = 0;    #5 $vogls_assert_eq(q, 1);
        // 0 -> x with the data equal to the state.
        clk = 1'bx; #5 $vogls_assert_eq(q, 1);
        // x -> 1 with the data equal to the state.
        clk = 1;    #5 $vogls_assert_eq(q, 1);
        // 1 -> x and x -> 0 hold.
        d = 0;      #5;
        clk = 1'bx; #5 $vogls_assert_eq(q, 1);
        clk = 0;    #5 $vogls_assert_eq(q, 1);
        // 0 -> z (as x) with the data not equal to the state.
        clk = 1'bz; #5 $vogls_assert_eq(q, 1'bx);
        clk = 0;    #5 $vogls_assert_eq(q, 1'bx);
        // 0 -> 1 loads the data.
        clk = 1;    #5 $vogls_assert_eq(q, 0);
        clk = 0;    #5;
        // x -> 1 with the data not equal to the state.
        d = 1;      #5;
        clk = 1'bx; #5 $vogls_assert_eq(q, 1'bx);
        clk = 1;    #5 $vogls_assert_eq(q, 1'bx);
        clk = 0;    #5 $vogls_assert_eq(q, 1'bx);
        // 0 -> 1 with an unknown data.
        d = 1'bx;   #5;
        clk = 1;    #5 $vogls_assert_eq(q, 1'bx);
        clk = 0;    #5;
        d = 0;      #5;
        clk = 1;    #5 $vogls_assert_eq(q, 0);
        // A changing data never changes the state.
        d = 1'bx;   #5 $vogls_assert_eq(q, 0);
        d = 1;      #5 $vogls_assert_eq(q, 0);
    end
endmodule
