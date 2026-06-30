module tb();
    reg x;
    wire a, b;

    not (a, x); 
    buf (b, x); 

    initial begin
        x = 0;
        #0
        $vogls_assert_eq(a, 1);
        $vogls_assert_eq(b, 0);

        x = 1;
        #0
        $vogls_assert_eq(a, 0);
        $vogls_assert_eq(b, 1);

`ifndef __VOGLS__TWO_VALUE_LOGIC
        x = 'bx;
        #0
        $vogls_assert_eq(a, 1'bx);
        $vogls_assert_eq(b, 1'bx);

        x = 'bz;
        #0
        $vogls_assert_eq(a, 1'bx);
        $vogls_assert_eq(b, 1'bx);
`endif
    end
endmodule
