module c0(o);
	output o;
	assign o = 1'b0;
endmodule

module c1(o);
	output o;
	assign o = 1'b1;
endmodule

module y(o);
	output [3:0] o;
	c0 z1(o[0]);
	c1 z2(o[1]);
	c0 z3(o[2]);
	c1 z4(o[3]);
endmodule

module x();
	wire [3:0] i;
    y y1(i);
	initial #1 $vogls_assert_eq(i, 4'b1010);
endmodule
