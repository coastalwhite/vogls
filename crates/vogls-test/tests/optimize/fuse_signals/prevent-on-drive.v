// vogls: verify-ir
// vogls: mode=template
module tb();
    reg a;
	wire x;
	assign x = a;
	assign x = $vogls_blackbox(1'b0);
endmodule
