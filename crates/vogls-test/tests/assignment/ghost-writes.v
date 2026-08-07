module x();
    reg [0:0] a, b, c;

    initial begin
        a = 0;
        b = 0;
        c = 0;
        $vogls_assert_eq(a, 0);
        $vogls_assert_eq(b, 0);
        $vogls_assert_eq(c, 0);

        #1 a[1] = 1'b1;
        $vogls_assert_eq(a, 0);
        $vogls_assert_eq(b, 0);
        $vogls_assert_eq(c, 0);
        
        #1 a[$vogls_blackbox(1)] = 1'b1;
        $vogls_assert_eq(a, 0);
        $vogls_assert_eq(b, 0);
        $vogls_assert_eq(c, 0);
        
        #1 b[1] = 1'b1;
        $vogls_assert_eq(a, 0);
        $vogls_assert_eq(b, 0);
        $vogls_assert_eq(c, 0);
        #1 b[$vogls_blackbox(1)] = 1'b1;
        $vogls_assert_eq(a, 0);
        $vogls_assert_eq(b, 0);
        $vogls_assert_eq(c, 0);
        
        #1 c[1] = 1'b1;
        $vogls_assert_eq(a, 0);
        $vogls_assert_eq(b, 0);
        $vogls_assert_eq(c, 0);
        #1 c[$vogls_blackbox(1)] = 1'b1;
        $vogls_assert_eq(a, 0);
        $vogls_assert_eq(b, 0);
        $vogls_assert_eq(c, 0);
        
        #1 a[-1] = 1'b1;
        $vogls_assert_eq(a, 0);
        $vogls_assert_eq(b, 0);
        $vogls_assert_eq(c, 0);
        #1 a[$vogls_blackbox(-1)] = 1'b1;
        $vogls_assert_eq(a, 0);
        $vogls_assert_eq(b, 0);
        $vogls_assert_eq(c, 0);
        
        #1 b[-1] = 1'b1;
        $vogls_assert_eq(a, 0);
        $vogls_assert_eq(b, 0);
        $vogls_assert_eq(c, 0);
        #1 b[$vogls_blackbox(-1)] = 1'b1;
        $vogls_assert_eq(a, 0);
        $vogls_assert_eq(b, 0);
        $vogls_assert_eq(c, 0);
        
        #1 c[-1] = 1'b1;
        $vogls_assert_eq(a, 0);
        $vogls_assert_eq(b, 0);
        $vogls_assert_eq(c, 0);
        #1 c[$vogls_blackbox(-1)] = 1'b1;
        $vogls_assert_eq(a, 0);
        $vogls_assert_eq(b, 0);
        $vogls_assert_eq(c, 0);
    end
endmodule
