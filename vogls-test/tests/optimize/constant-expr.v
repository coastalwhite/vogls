module x();
	initial begin
        if (|32'h010101) $display("true!");
        else             $display("false!");
	end
endmodule
