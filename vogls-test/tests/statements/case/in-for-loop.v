// vogls: verify-stdout
module x();
    integer i, j;
    initial begin
		j = 0;
		for (i = 0; i < 10; i = i + 1) begin
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

			$display(j);
		end
		$vogls_assert_eq(j, 1996);
    end
endmodule
