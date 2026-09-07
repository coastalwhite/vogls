module tb();
    reg [127:0] x = 5;
    initial $vogls_assert_eq($vogls_slice($vogls_blackbox(x), 63, 7), 0);
endmodule
