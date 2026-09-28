// vogls: verify-stdout
`timescale 1ps / 1fs
module x();
    integer a;
	initial begin
        $display("Gonna start waiting.");
        wait (a > 5) $display("Wow, we reached it at %0d!", $time);
        $display("Now we can go to rest.");
	end

	initial begin
        #0 a = 1;
        #1 a = 2;
        #2 a = 3;
        #1 a = 4;
        #2 a = 5;
        #1 a = 6;
        #2 a = 7;
        #1 a = 8;
	end
endmodule
