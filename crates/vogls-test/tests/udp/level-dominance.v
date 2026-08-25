primitive leveldom(q, a, b);
    output q; reg q;
    input a, b;
table
    f ? : ? : 0 ;   // edge-sensitive, listed first
    ? 1 : ? : 1 ;   // level-sensitive, must dominate
endtable
endprimitive

module tb();
    reg a, b;
    wire q;

    leveldom (q, a, b);

    initial begin
        #0 a = 1; b = 1;
        #5 a = 0;
        #5 $vogls_assert_eq(q, 1);
    end
endmodule
