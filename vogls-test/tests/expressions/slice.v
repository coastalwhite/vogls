module x();
    initial begin
        $vogls_assert_eq((33'h0204_0608)[7:0], 8'h08);
        $vogls_assert_eq((33'h0204_0608)[11:0], 12'h608);
        $vogls_assert_eq((33'h0204_0608)[31:0], 32'h0204_0608);
        $vogls_assert_eq((33'h0204_0608)[31:4], 28'h0204_060);
        $vogls_assert_eq((33'h0204_0608)[28:1], 28'h0102_0304);
        $vogls_assert_eq((33'h0204_0608)[30:3], 28'h0040_80C1);
    end
endmodule
