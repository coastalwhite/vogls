module x();
    initial begin
        $vogls_assert_eq((33'h0204_0608)[7:0], 8'h08);
        $vogls_assert_eq((33'h0204_0608)[11:0], 12'h608);
        $vogls_assert_eq((33'h0204_0608)[31:0], 32'h0204_0608);
    end
endmodule
