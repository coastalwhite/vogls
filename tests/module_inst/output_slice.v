module z(o);
	output [1:0] o;
	assign o = 2'b10;
endmodule

module y(o);
	output [3:0] o;
	z z1(o[1:0]);
	z z2(o[3:2]);
endmodule

module x();
	wire [3:0] i;
    y y1(i);
	initial #1 $vogls_assert_eq(i, 4'b1010);
endmodule
