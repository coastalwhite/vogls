module x();
    parameter X = 16'h 1234;
    parameter Y = 'b 1010;

    initial begin
        $vogls_assert_eq(X, 16'h1234);
        $vogls_assert_eq(Y, 4'b1010);
    end
endmodule
