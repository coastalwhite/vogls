module z(i, o);
    input  [1:0] i;
    output [1:0] o;
    assign o = { i[0], i[1] };
endmodule

module y(i, o);
    input  [1:0] i;
    output [1:0] o;
    assign o = { i[0], i[1] };
endmodule

module x();
	reg [2:0] i;
	wire [1:0] imm;
	wire [1:0] o;
	y y1(i[1:0], imm);
	z z1(imm, o);

	integer j;
	initial begin
		for (j = 0; j < 8; j = j + 1) begin
			#1 i = j;
			#1 $vogls_assert_eq(o, i[1:0]);
		end
	end
endmodule
