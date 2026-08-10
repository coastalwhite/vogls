module x();
    real x = 1.5;
    real z = 2.25;

    real a = 2.0;
    real b = 8.0;

    initial begin
        $vogls_assert_eq(x + z, 3.75);
        $vogls_assert_eq(z - x, 0.75);
        $vogls_assert_eq(x * a, 3.0);
        $vogls_assert_eq(b / a, 4.0);
        $vogls_assert_eq(a ** 3.0, 8.0);

        $vogls_assert_eq(-x, -1.5);
        $vogls_assert_eq(-(x + z), -3.75);

        $vogls_assert_eq(z > x, 1);
        $vogls_assert_eq(x > z, 0);
        $vogls_assert_eq(a >= a, 1);
        $vogls_assert_eq(x < z, 1);
        $vogls_assert_eq(a <= a, 1);
        $vogls_assert_eq(x == 1.5, 1);
        $vogls_assert_eq(x != z, 1);

        $vogls_assert_eq(a && b, 1);
        $vogls_assert_eq(0.0 && b, 0);
        $vogls_assert_eq(0.0 || b, 1);
        $vogls_assert_eq(0.0 || 0.0, 0);

        $vogls_assert_eq(x + 2, 3.5);
        $vogls_assert_eq(3 / a, 1.5);
        $vogls_assert_eq(a == 2, 1);
        $vogls_assert_eq(a * 3, 6.0);
    end
endmodule
