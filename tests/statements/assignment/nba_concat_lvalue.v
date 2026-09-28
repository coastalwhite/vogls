module x();
	reg [31:0] y;
	initial begin
		{ y[31:20], y[10:1], y[11], y[19:12], y[0] } <= 32'ShFFFFEBFE;
		#1
		$vogls_assert_eq(y, 32'ShFFFFFFF4);
	end
endmodule
