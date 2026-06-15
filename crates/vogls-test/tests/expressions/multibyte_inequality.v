// vogls: verify-stdout
module x();
	integer i;
	initial begin
	    for (i = 0; i < 256; i = i + 1) begin
			$display("hi %0x!", i);
		end
	end
endmodule
