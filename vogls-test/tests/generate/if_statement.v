// vogls: verify-stdout
module x();
    if (1'b0) begin
        initial $display("Should not be printed");
        initial $vogls_assert_eq(1'b0, 1'b1);
    end
    if (1'b1) begin
        initial $display("Should be printed");
        initial $vogls_assert_eq(1'b1, 1'b1);
    end
endmodule
