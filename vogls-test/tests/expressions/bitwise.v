module tb();
    initial begin
        $vogls_assert_eq(1'b0 & 1'b0, 1'b0);
        $vogls_assert_eq(1'b1 & 1'b0, 1'b0);
        $vogls_assert_eq(1'b0 & 1'b1, 1'b0);
        $vogls_assert_eq(1'b1 & 1'b1, 1'b1);
`ifndef __VOGLS__TWO_VALUE_LOGIC
        $vogls_assert_eq(1'bx & 1'b0, 1'b0);
        $vogls_assert_eq(1'bx & 1'b1, 1'bx);
        $vogls_assert_eq(1'bx & 1'bx, 1'bx);
        $vogls_assert_eq(1'bx & 1'bz, 1'bx);

        $vogls_assert_eq(1'bz & 1'b0, 1'b0);
        $vogls_assert_eq(1'bz & 1'b1, 1'bx);
        $vogls_assert_eq(1'bz & 1'bx, 1'bx);
        $vogls_assert_eq(1'bz & 1'bz, 1'bx);

        $vogls_assert_eq(1'b0 & 1'bx, 1'b0);
        $vogls_assert_eq(1'b1 & 1'bx, 1'bx);
        $vogls_assert_eq(1'b0 & 1'bz, 1'b0);
        $vogls_assert_eq(1'b1 & 1'bz, 1'bx);
`endif

        $vogls_assert_eq(1'b0 | 1'b0, 1'b0);
        $vogls_assert_eq(1'b1 | 1'b0, 1'b1);
        $vogls_assert_eq(1'b0 | 1'b1, 1'b1);
        $vogls_assert_eq(1'b1 | 1'b1, 1'b1);
`ifndef __VOGLS__TWO_VALUE_LOGIC
        $vogls_assert_eq(1'bx | 1'b0, 1'bx);
        $vogls_assert_eq(1'bx | 1'b1, 1'b1);
        $vogls_assert_eq(1'bx | 1'bx, 1'bx);
        $vogls_assert_eq(1'bx | 1'bz, 1'bx);

        $vogls_assert_eq(1'bz | 1'b0, 1'bx);
        $vogls_assert_eq(1'bz | 1'b1, 1'b1);
        $vogls_assert_eq(1'bz | 1'bx, 1'bx);
        $vogls_assert_eq(1'bz | 1'bz, 1'bx);

        $vogls_assert_eq(1'b0 | 1'bx, 1'bx);
        $vogls_assert_eq(1'b1 | 1'bx, 1'b1);
        $vogls_assert_eq(1'b0 | 1'bz, 1'bx);
        $vogls_assert_eq(1'b1 | 1'bz, 1'b1);
`endif

        $vogls_assert_eq(1'b0 ^ 1'b0, 1'b0);
        $vogls_assert_eq(1'b1 ^ 1'b0, 1'b1);
        $vogls_assert_eq(1'b0 ^ 1'b1, 1'b1);
        $vogls_assert_eq(1'b1 ^ 1'b1, 1'b0);
`ifndef __VOGLS__TWO_VALUE_LOGIC
        $vogls_assert_eq(1'bx ^ 1'b0, 1'bx);
        $vogls_assert_eq(1'bx ^ 1'b1, 1'bx);
        $vogls_assert_eq(1'bx ^ 1'bx, 1'bx);
        $vogls_assert_eq(1'bx ^ 1'bz, 1'bx);

        $vogls_assert_eq(1'bz ^ 1'b0, 1'bx);
        $vogls_assert_eq(1'bz ^ 1'b1, 1'bx);
        $vogls_assert_eq(1'bz ^ 1'bx, 1'bx);
        $vogls_assert_eq(1'bz ^ 1'bz, 1'bx);

        $vogls_assert_eq(1'b0 ^ 1'bx, 1'bx);
        $vogls_assert_eq(1'b1 ^ 1'bx, 1'bx);
        $vogls_assert_eq(1'b0 ^ 1'bz, 1'bx);
        $vogls_assert_eq(1'b1 ^ 1'bz, 1'bx);
`endif
    end
endmodule
