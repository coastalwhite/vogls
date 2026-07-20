module tb();
    initial begin $vogls_assert_eq((1'b1 + 1'b1 + 1'b1) >= 2, 1'b1);
endmodule
