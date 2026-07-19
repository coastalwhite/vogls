module test;
    integer uniform     = 0,
            normal      = 0,
            exponential = 0,
            poisson     = 0,
            chi_square  = 0,
            t           = 0,
            erlang      = 0;

    initial begin
        #0
        $vogls_assert_eq($random(uniform), 303379748);
        $vogls_assert_eq($dist_uniform(uniform, -52, 1000), 213);
        $vogls_assert_eq($dist_uniform(uniform, -17, 999), 0);

        $vogls_assert_eq($dist_normal(normal, -52, 1000), 394);
        $vogls_assert_eq($dist_normal(normal, -17, 999), 128);

        $vogls_assert_eq($dist_exponential(exponential, 1000), 561);
        $vogls_assert_eq($dist_exponential(exponential, -17), 0);

        $vogls_assert_eq($dist_chi_square(chi_square, 1), 0);
        $vogls_assert_eq($dist_chi_square(chi_square, 5), 10);

        $vogls_assert_eq($dist_t(t, 1), 0);
        $vogls_assert_eq($dist_t(t, 17), 1);

        $vogls_assert_eq($dist_erlang(erlang, 1, 99), 56);
        $vogls_assert_eq($dist_erlang(erlang, 5, 1000), 1589);
    end
endmodule
