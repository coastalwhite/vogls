// vogls: verify-stdout
module x();
    integer i, j;
    initial begin
		j = 0;
		for (i = 0; i < 10; i = i + 1) begin
			if (i == 0)      j = 13;
			else if (i == 1) j = 37;
			else if (i == 2) j = 420;
			else if (i == 3) j = 360;
			else if (i == 4) j = 42;
			else if (i == 5) j = 15;
			else if (i == 6) j = 09;
			else if (i == 7) j = 2000;
			else             j = 1996;

			$display(j);
		end
    end
endmodule
