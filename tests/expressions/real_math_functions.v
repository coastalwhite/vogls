module x();
    real a = 9.0;
    real b = 2.25;

    initial begin
        $vogls_assert_eq($exp(0.0), 1.0);
        $vogls_assert_eq($ln(1.0), 0.0);
        $vogls_assert_eq($log10(1.0), 0.0);

        $vogls_assert_eq($sqrt(9.0), 3.0);
        $vogls_assert_eq($sqrt(16.0), 4.0);
        $vogls_assert_eq($sqrt(0.25), 0.5);
        $vogls_assert_eq($sqrt(b), 1.5);
        $vogls_assert_eq($sqrt(a), 3.0);

        $vogls_assert_eq($floor(2.75), 2.0);
        $vogls_assert_eq($ceil(2.25), 3.0);
        $vogls_assert_eq($floor(-1.5), -2.0);
        $vogls_assert_eq($ceil(-1.5), -1.0);

        $vogls_assert_eq($sin(0.0), 0.0);
        $vogls_assert_eq($cos(0.0), 1.0);
        $vogls_assert_eq($tan(0.0), 0.0);
        $vogls_assert_eq($asin(0.0), 0.0);
        $vogls_assert_eq($acos(1.0), 0.0);
        $vogls_assert_eq($atan(0.0), 0.0);

        $vogls_assert_eq($sinh(0.0), 0.0);
        $vogls_assert_eq($cosh(0.0), 1.0);
        $vogls_assert_eq($tanh(0.0), 0.0);
        $vogls_assert_eq($asinh(0.0), 0.0);
        $vogls_assert_eq($acosh(1.0), 0.0);
        $vogls_assert_eq($atanh(0.0), 0.0);

        $vogls_assert_eq($atan2(0.0, 1.0), 0.0);
        $vogls_assert_eq($hypot(3.0, 4.0), 5.0);
        $vogls_assert_eq($hypot(5.0, 12.0), 13.0);

        $vogls_assert_eq($sqrt(9), 3.0);
        $vogls_assert_eq($floor(3), 3.0);
        $vogls_assert_eq($hypot(3, 4), 5.0);
    end
endmodule
