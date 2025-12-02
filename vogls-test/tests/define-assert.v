`define X 1
`define Y 0

module x();
    initial begin
        $vogls_assert_eq(`X, 1);
        $vogls_assert_eq(`Y, 0);
    end
endmodule
