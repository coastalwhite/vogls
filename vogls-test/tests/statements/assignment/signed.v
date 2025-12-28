module x();
    wire [7:0] y;
    initial begin
        // Blocking Assignment
        y = $signed(4'h8);
        $vogls_assert_eq(y, 8'hF8);
        #1
        y = 4'h8;
        $vogls_assert_eq(y, 8'h08);
        #1

        // Non-Blocking Assignment
        y <= $signed(4'h8);
        #1 $vogls_assert_eq(y, 8'hF8);
        #1
        y <= 4'h8;
        #1 $vogls_assert_eq(y, 8'h08);
    end
endmodule
