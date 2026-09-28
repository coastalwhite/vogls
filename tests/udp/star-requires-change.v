primitive starchk(q, a, b);
    output q; reg q;
    input a, b;
table
    ? * : ? : 1 ;   // only when b changes
    * ? : ? : 0 ;   // only when a changes
endtable
endprimitive

module tb();
    reg a, b;
    wire q;

    starchk (q, a, b);

    initial begin
        #0 a = 0; b = 0;
        #5

        a = 1; #5 $vogls_assert_eq(q, 0);
        b = 1; #5 $vogls_assert_eq(q, 1);
    end
endmodule
