module tb();
    wire [2:9] a = 8'hA3;
    initial begin
        $vogls_assert_eq(a, 8'hA3);
        $vogls_assert_eq(a[2], 1'b1);
        $vogls_assert_eq(a[3], 1'b0);
        $vogls_assert_eq(a[4], 1'b1);
        $vogls_assert_eq(a[5], 1'b0);
        $vogls_assert_eq(a[6], 1'b0);
        $vogls_assert_eq(a[7], 1'b0);
        $vogls_assert_eq(a[8], 1'b1);
        $vogls_assert_eq(a[9], 1'b1);
    end
endmodule
