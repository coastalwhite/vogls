module x();
    initial begin
        $vogls_assert_eq(1'b0 ? 8'h13 : 8'h42, 8'h42);
        $vogls_assert_eq(1'b1 ? 8'h13 : 8'h42, 8'h13);

        $vogls_assert_eq(1'b0 ? 8'h13 : 1'b0 ? 8'h13 : 8'h42, 8'h42);
        $vogls_assert_eq(1'b0 ? 8'h13 : 1'b1 ? 8'h15 : 8'h42, 8'h15);
        $vogls_assert_eq(1'b1 ? 8'h13 : 1'b1 ? 8'h15 : 8'h42, 8'h13);

        $vogls_assert_eq(1'b0 ? 8'h13 : 1'b0 ? 8'h13 : 1'b0 ? 8'h13 : 8'h42, 8'h42);
    end
endmodule
