module x();
    initial begin
        $vogls_assert_eq(2'b01[0], 1'b1);
        $vogls_assert_eq(2'b01[1], 1'b0);

        $vogls_assert_eq({ 8'hFF, 56'h0 }[58], 1'b1);
        $vogls_assert_eq({ 8'hFF, 56'h0 }[56], 1'b1);
        $vogls_assert_eq({ 8'hFF, 56'h0 }[55], 1'b0);
    end
endmodule
