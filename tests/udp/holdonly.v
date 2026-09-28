primitive holdonly(q, clk, d);
    output q; reg q;
    input clk, d;
    initial q = 1'b1;
table
    ? ? : ? : - ;
endtable
endprimitive

module tb();
    reg clk, d;
    wire q;

    holdonly (q, clk, d);

    initial begin
        #0 $vogls_assert_eq(q, 1);
           clk = 0; d = 0;
        #5 $vogls_assert_eq(q, 1);
           clk = 1; d = 1;
        #5 $vogls_assert_eq(q, 1);
    end
endmodule
