module top_module ( 
    input p1a, p1b, p1c, p1d,
    output p1y,
    input p2a, p2b, p2c, p2d,
    output p2y );

    assign p1y = ~(p1a & p1b & p1c & p1d);
    assign p2y = ~(p2a & p2b & p2c & p2d);
endmodule

module tb();
    reg p1a, p1b, p1c, p1d;
    reg p2a, p2b, p2c, p2d;
    wire p1y, p2y;

    top_module i(
        p1a, p1b, p1c, p1d,
        p1y,
        p2a, p2b, p2c, p2d,
        p2y
    );

    initial begin
        p1a = 0; p1b = 0; p1c = 0; p1d = 0;
        p2a = 0; p2b = 0; p2c = 0; p2d = 0;
        #1
        $vogls_assert_eq(p1y, 1); $vogls_assert_eq(p2y, 1);

        p1a = 1; p1b = 0; p1c = 1; p1d = 0;
        p2a = 0; p2b = 1; p2c = 0; p2d = 1;
        #1
        $vogls_assert_eq(p1y, 1); $vogls_assert_eq(p2y, 1);

        p1a = 0; p1b = 1; p1c = 0; p1d = 1;
        p2a = 1; p2b = 0; p2c = 1; p2d = 0;
        #1
        $vogls_assert_eq(p1y, 1); $vogls_assert_eq(p2y, 1);

        p1a = 1; p1b = 1; p1c = 1; p1d = 1;
        p2a = 1; p2b = 0; p2c = 1; p2d = 0;
        #1
        $vogls_assert_eq(p1y, 0); $vogls_assert_eq(p2y, 1);

        p1a = 1; p1b = 1; p1c = 1; p1d = 1;
        p2a = 1; p2b = 1; p2c = 1; p2d = 1;
        #1
        $vogls_assert_eq(p1y, 0); $vogls_assert_eq(p2y, 0);

        p1a = 1; p1b = 0; p1c = 1; p1d = 0;
        p2a = 1; p2b = 1; p2c = 1; p2d = 1;
        #1
        $vogls_assert_eq(p1y, 1); $vogls_assert_eq(p2y, 0);
    end
endmodule
