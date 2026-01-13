// vogls: verify-stdout
module x();
	genvar i;
	for (i = 0; i < 5; i = i + 1) initial #i $display("Print %0d", i);
endmodule
