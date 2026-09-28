primitive jk_edge_ff(q, clock, j, k, preset, clear);
    output q; reg q;
    input clock, j, k, preset, clear;
table
//    clock  jk   pc    state  output/next state
      ?      ??   01  : ?    : 1 ;   // preset logic
      ?      ??   *1  : 1    : 1 ;
      ?      ??   10  : ?    : 0 ;   // clear logic
      ?      ??   1*  : 0    : 0 ;
      r      00   00  : 0    : 1 ;   // normal clocking cases
      r      00   11  : ?    : - ;
      r      01   11  : ?    : 0 ;
      r      10   11  : ?    : 1 ;
      r      11   11  : 0    : 1 ;
      r      11   11  : 1    : 0 ;
      f      ??   ??  : ?    : - ;
      b      *?   ??  : ?    : - ;   // j and k transition cases
      b      ?*   ??  : ?    : - ;
endtable
endprimitive

module tb();
    reg clock, j, k, preset, clear;
    wire q;

    jk_edge_ff (q, clock, j, k, preset, clear);

    initial begin
        #0 clock = 0; j = 0; k = 0; preset = 0; clear = 1;
        #5 $vogls_assert_eq(q, 1);          // preset
           preset = 1; clear = 0;
        #5 $vogls_assert_eq(q, 0);          // clear
           preset = 1; clear = 1; j = 1; k = 0;
        #5 clock = 1;                       // rising edge, jk = 10 -> set
        #5 $vogls_assert_eq(q, 1);
           clock = 0; j = 0; k = 1;
        #5 clock = 1;                       // rising edge, jk = 01 -> reset
        #5 $vogls_assert_eq(q, 0);
           clock = 0; j = 1; k = 1;
        #5 clock = 1;                       // rising edge, jk = 11 -> toggle
        #5 $vogls_assert_eq(q, 1);
    end
endmodule
