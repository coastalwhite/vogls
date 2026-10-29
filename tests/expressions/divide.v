module x();
    initial begin
        $vogls_assert_eq(2 / 2, 1);
        $vogls_assert_eq(8 / 3, 2);
        $vogls_assert_eq(10 / 1, 10);
        $vogls_assert_eq($vogls_div0($vogls_blackbox(10), $vogls_blackbox(3)), 3);
        $vogls_assert_eq($vogls_mod0($vogls_blackbox(10), $vogls_blackbox(3)), 1);
    end
endmodule
