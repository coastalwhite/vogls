module x();
    initial begin
        // $vogls_assert_eq({ 1'b1, 1'b0 }, 2'b10);
        // $vogls_assert_eq({ 1'b0, 1'b1 }, 2'b01);

        $vogls_assert_eq({ 16'hFFFF, 12'hFFF }, 28'hFFFF_FFF);
        // $vogls_assert_eq({ 32'h0102_0304, 1'b0 }, 33'h0204_0608);
        // $vogls_assert_eq({ 15'h0081, 18'h0608 }, 33'h0204_0608);
    end
endmodule
