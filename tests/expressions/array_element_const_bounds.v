module tb;
	reg [7:0] mem [0:3];
	reg [7:0] r;
	integer   changes;

	always @(r) changes = changes + 1;

	initial begin
		mem[0] = 8'h11; mem[1] = 8'h22; mem[2] = 8'h33; mem[3] = 8'h44;
		r = 8'h00;
		changes = 0;
		#1;
		changes = 0;

		mem[1][11:4] = 8'hff;
		$vogls_assert_eq(mem[0], 8'h11);
		$vogls_assert_eq(mem[1], 8'hf2);
		$vogls_assert_eq(mem[2], 8'h33);
		$vogls_assert_eq(mem[3], 8'h44);

		mem[2][3:-4] = 8'h5a;
		$vogls_assert_eq(mem[1], 8'hf2);
		$vogls_assert_eq(mem[2], 8'h35);
		$vogls_assert_eq(mem[3], 8'h44);

		mem[1][19:12] = 8'h00;
		$vogls_assert_eq(mem[1], 8'hf2);
		$vogls_assert_eq(mem[2], 8'h35);
		$vogls_assert_eq(mem[3], 8'h44);

		mem[2][5:2] = 4'h0;
		$vogls_assert_eq(mem[2], 8'h01);
		$vogls_assert_eq(mem[3], 8'h44);

		r[11:4] = 8'hf0;
		#1;
		$vogls_assert_eq(r, 8'h00);
		$vogls_assert_eq(changes, 0);
		r[11:4] = 8'hf1;
		#1;
		$vogls_assert_eq(r, 8'h10);
		$vogls_assert_eq(changes, 1);
		r[19:12] = 8'hff;
		#1;
		$vogls_assert_eq(r, 8'h10);
		$vogls_assert_eq(changes, 1);
	end
endmodule
