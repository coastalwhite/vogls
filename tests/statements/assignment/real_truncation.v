module tb;
    real r; integer i;
    initial begin
        r=2.7; i=r;  $vogls_assert_eq(i, 3);
        r=2.5; i=r;  $vogls_assert_eq(i, 3);
        r=-2.5;i=r;  $vogls_assert_eq(i, -3);
        r=3.99;i=r;  $vogls_assert_eq(i, 4);
    end
endmodule
