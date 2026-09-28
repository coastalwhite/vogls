module x();
    initial $vogls_assert_eq(2 % 2, 0);
    initial $vogls_assert_eq(8 % 3, 2);
    initial $vogls_assert_eq(10 % 1, 0);
    initial $vogls_assert_eq(10 % 7, 3);
endmodule
