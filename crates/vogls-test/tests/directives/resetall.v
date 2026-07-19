// vogls: verify-stdout
`timescale 1ps/1ps
`resetall

module tb();
	initial $printtimescale();
endmodule
