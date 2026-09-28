module x();
    integer i;
    initial begin
        i = 0;
        if (1'b0) i = 1;
        else      i = 2;
        $vogls_assert_eq(i, 2);

        if (1'b1) i = 1;
        else      i = 2;
        $vogls_assert_eq(i, 1);
    end
endmodule
