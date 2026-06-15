module top_module(
    input clk,
    input a, 
    input b,
    output wire out_assign,
    output reg out_always_comb,
    output reg out_always_ff
);
    assign out_assign = a ^ b;
    always @(*) out_always_comb = a ^ b;
    always @(posedge clk) out_always_ff <= a ^ b;
endmodule

module tb();
    reg a, b, clk;
    wire out_assign, out_always_comb, out_always_ff;

    top_module m(clk, a, b, out_assign, out_always_comb, out_always_ff);

    always #5 clk = ~clk;
    initial begin
        clk = 0;
        #1 a = 0; b = 0;
        #5
        $vogls_assert_eq(out_assign, 0);
        $vogls_assert_eq(out_always_comb, 0);
        $vogls_assert_eq(out_always_ff, 0);

        #5 a = 0; b = 1;
        #5
        $vogls_assert_eq(out_assign, 1);
        $vogls_assert_eq(out_always_comb, 1);
        $vogls_assert_eq(out_always_ff, 1);

        #5 a = 1; b = 1;
        #5
        $vogls_assert_eq(out_assign, 0);
        $vogls_assert_eq(out_always_comb, 0);
        $vogls_assert_eq(out_always_ff, 0);

        #5 a = 1; b = 0;
        #5
        $vogls_assert_eq(out_assign, 1);
        $vogls_assert_eq(out_always_comb, 1);
        $vogls_assert_eq(out_always_ff, 1);

        $finish();
    end
endmodule
