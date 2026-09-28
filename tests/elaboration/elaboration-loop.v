// vogls: fail=elaborate
module tb();
	parameter X = Y;
	parameter Y = Z;
	parameter Z = X;
endmodule
