module tb();
    wire [7:0] a = 8'hA3;
    wire [0:7] b;
    assign b = a;
    initial #0 $vogls_assert_eq(b, 8'hA3);
endmodule
