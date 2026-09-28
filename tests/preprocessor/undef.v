`define X & 0

module x();
    initial begin
        $vogls_assert_eq(0, 1 `X);
        `undef X
        $vogls_assert_eq(1, 1 `X);
    end
endmodule
