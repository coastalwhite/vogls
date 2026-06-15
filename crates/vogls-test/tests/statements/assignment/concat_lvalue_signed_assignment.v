module x();
    wire [7:0] y;
    initial begin
        // Blocking Assignment
        { y[0], y[7:1] } = $signed(4'h8);
        $vogls_assert_eq(y, 8'hF1);
        #1
        { y[0], y[7:1] } = 4'h8;
        $vogls_assert_eq(y, 8'h10);

        #1

        // Non-Blocking Assignment
        { y[0], y[7:1] } <= $signed(4'h8);
        #1 $vogls_assert_eq(y, 8'hF1);

        { y[0], y[7:1] } <= 4'h8;
        #1 $vogls_assert_eq(y, 8'h10);
    end
endmodule
