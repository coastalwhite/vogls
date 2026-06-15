module x();
    integer i, j;
    initial begin
        i = 4;
		j = 0;
        case (i)
            0:       j = 13;
            1:       j = 37;
            2:       j = 420;
            3:       j = 360;
            4:       j = 42;
            5:       j = 15;
            6:       j = 09;
            7:       j = 2000;
            default: j = 1996;
        endcase
        $vogls_assert_eq(j, 42);

        i = 2;
        case (i)
            0:       j = 13;
            1:       j = 37;
            2:       j = 420;
            3:       j = 360;
            4:       j = 42;
            5:       j = 15;
            6:       j = 09;
            7:       j = 2000;
            default: j = 1996;
        endcase
        $vogls_assert_eq(j, 420);
    end
endmodule
