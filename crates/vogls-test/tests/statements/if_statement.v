module x();
    integer i;
    initial begin
        i = 0;
        $vogls_assert_eq(i, 0);
        if (1'b1) i = 1;
        $vogls_assert_eq(i, 1);
    end
endmodule
