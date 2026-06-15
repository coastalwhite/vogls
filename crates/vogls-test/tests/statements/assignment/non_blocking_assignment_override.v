module x();
    wire y;

    initial begin
        y <= 0;
        y <= 1;
        #1
        $vogls_assert_eq(y, 1'b1);
    end
endmodule
