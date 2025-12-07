module x();
    initial begin
        $vogls_assert_eq(2'b01[0], 1);
        $vogls_assert_eq(2'b01[1], 0);
    end
endmodule
