module x();
    integer i, j;
    initial begin
		i = 0;
        j = 13;
        case (i)
            1: j = 42;
        endcase
        $vogls_assert_eq(j, 13);
    end
endmodule
