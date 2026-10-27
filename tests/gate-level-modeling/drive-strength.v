module tb();
    reg a;
    wire o1, o2, o3, o4;

    buf (weak0, weak1)            (o1, a);
    not (strong1, pull0)          (o2, a);
    and (supply0, strong1) #1     (o3, a, a);
    or  (pull1, weak0)         g4 (o4, a, a);

    // @TODO: `highz0` and `highz1` should make the gate drive z instead of 0 or 1. These are only
    // parsed for now.
    wire h1, h2, h3, h4;
    buf (strong0, highz1) (h1, a);
    buf (strong1, highz0) (h2, a);
    buf (highz0, strong1) (h3, a);
    buf (highz1, strong0) (h4, a);

    initial begin
        a = 0;
        #2
        $vogls_assert_eq(o1, 0);
        $vogls_assert_eq(o2, 1);
        $vogls_assert_eq(o3, 0);
        $vogls_assert_eq(o4, 0);

        a = 1;
        #2
        $vogls_assert_eq(o1, 1);
        $vogls_assert_eq(o2, 0);
        $vogls_assert_eq(o3, 1);
        $vogls_assert_eq(o4, 1);
    end
endmodule
