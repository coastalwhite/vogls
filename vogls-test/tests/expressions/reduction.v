module tb();
    initial begin
        // $vogls_assert_eq(^8'b0000_0000, 0);
        // $vogls_assert_eq(^8'b0000_0001, 1);
        // $vogls_assert_eq(^8'b1000_0001, 0);
        // $vogls_assert_eq(^8'b1001_1001, 0);
        // $vogls_assert_eq(^8'b1011_1111, 1);
        // $vogls_assert_eq(^8'b1111_1111, 0);
        // $vogls_assert_eq(^9'b1111_1111, 0);
        // $vogls_assert_eq(^9'b1_1111_1111, 1);
        // $vogls_assert_eq(^9'b0_0000_0000, 0);
        // $vogls_assert_eq(^129'h0_89ABCDEF_12345678_A37BF258_12398791, 0);
        $vogls_assert_eq(^129'h0_89ABCDEF_13345678_A37BF258_12398791, 1);
        // $vogls_assert_eq(^129'h0_0000000_00000000_00000000_00000000, 0);
        //
        // $vogls_assert_eq(|8'b0000_0000, 0);
        // $vogls_assert_eq(|8'b0000_0001, 1);
        // $vogls_assert_eq(|8'b1000_0001, 1);
        // $vogls_assert_eq(|8'b1001_1001, 1);
        // $vogls_assert_eq(|8'b1011_1111, 1);
        // $vogls_assert_eq(|8'b1111_1111, 1);
        // $vogls_assert_eq(|9'b1111_1111, 1);
        // $vogls_assert_eq(|9'b1_1111_1111, 1);
        // $vogls_assert_eq(|9'b0_0000_0000, 0);
        // $vogls_assert_eq(|129'h0_89ABCDEF_12345678_A37BF258_12398791, 1);
        // $vogls_assert_eq(|129'h0_89ABCDEF_13345678_A37BF258_12398791, 1);
        // $vogls_assert_eq(|129'h0_00000000_00000000_00000000_00000000, 0);
        // $vogls_assert_eq(|129'h1_FFFFFFFF_FFFFFFFF_FFFFFFFF_FFFFFFFF, 1);
        //
        // $vogls_assert_eq(&8'b0000_0000, 0);
        // $vogls_assert_eq(&8'b0000_0001, 0);
        // $vogls_assert_eq(&8'b1000_0001, 0);
        // $vogls_assert_eq(&8'b1001_1001, 0);
        // $vogls_assert_eq(&8'b1011_1111, 0);
        // $vogls_assert_eq(&8'b1111_1111, 1);
        // $vogls_assert_eq(&9'b1111_1111, 0);
        // $vogls_assert_eq(&9'b1_1111_1111, 1);
        // $vogls_assert_eq(&9'b0_0000_0000, 0);
        // $vogls_assert_eq(&129'h0_89ABCDEF_12345678_A37BF258_12398791, 0);
        // $vogls_assert_eq(&129'h0_89ABCDEF_13345678_A37BF258_12398791, 0);
        // $vogls_assert_eq(&129'h0_00000000_00000000_00000000_00000000, 0);
        // $vogls_assert_eq(&129'h1_FFFFFFFF_FFFFFFFF_FFFFFFFF_FFFFFFFF, 1);
    end
endmodule
