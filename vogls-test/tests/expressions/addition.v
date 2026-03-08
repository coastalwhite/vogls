module x();
    initial begin
        $vogls_assert_eq(1'b0 + 1'b0, 1'b0);
        $vogls_assert_eq(1'b1 + 1'b0, 1'b1);
        $vogls_assert_eq(1'b0 + 1'b1, 1'b1);
        $vogls_assert_eq(1'b1 + 1'b1, 1'b0);
`ifndef __VOGLS__TWO_VALUE_LOGIC
        $vogls_assert_eq(1'bx + 1'b0, 1'bx);
        $vogls_assert_eq(1'bz + 1'b0, 1'bx);
        $vogls_assert_eq(1'bx + 1'b1, 1'bx);
        $vogls_assert_eq(1'bz + 1'b1, 1'bx);
        $vogls_assert_eq(1'b0 + 1'bx, 1'bx);
        $vogls_assert_eq(1'b0 + 1'bz, 1'bx);
        $vogls_assert_eq(1'b1 + 1'bx, 1'bx);
        $vogls_assert_eq(1'b1 + 1'bz, 1'bx);
`endif

        $vogls_assert_eq(5'b0 + 5'b0, 5'b0);
        $vogls_assert_eq(5'b1 + 5'b0, 5'b1);
        $vogls_assert_eq(5'b0 + 5'b1, 5'b1);
        $vogls_assert_eq(5'b1 + 5'b1, 5'b0);
`ifndef __VOGLS__TWO_VALUE_LOGIC
        $vogls_assert_eq(5'bx1011 + 5'b00000, 5'bx);
        $vogls_assert_eq(5'b101z1 + 5'b11111, 5'bx);
        $vogls_assert_eq(5'bx101z + 5'b10xz1, 5'bx);
        $vogls_assert_eq(5'bxz101 + 5'bxz111, 5'bx);
        $vogls_assert_eq(5'b10101 + 5'bxzxzx, 5'bx);
`endif
    end
endmodule
