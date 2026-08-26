// vogls: fail=execute
primitive latch1(output reg q = 1'b1, input clk, input d);
table
    0 1 : ? : 1 ;
    0 0 : ? : 0 ;
    1 ? : ? : - ;
endtable
endprimitive

module tb();
    reg clk, d;
    wire q;

    latch1 (q, clk, d);

    initial begin
        #0 clk = 1; d = 0;       // latch closed, so q keeps its initial value
        #5 $vogls_assert_eq(q, 1);
    end
endmodule
