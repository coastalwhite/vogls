// A part-select of an array element with a dynamic index is bounded by that element. Bits past
// the element read as X (0 in two-value logic) and are never written, so neighbouring elements
// stay untouched.
module tb;
	reg [15:0]  mem  [0:3];
	reg [127:0] wmem [0:2];
	reg         r;
	integer     edges;

	always @(posedge r) edges = edges + 1;

	initial begin
		mem[0] = 16'h1111; mem[1] = 16'h2222; mem[2] = 16'h3333; mem[3] = 16'h4444;
		wmem[0] = {4{32'h0000_0000}};
		wmem[1] = {4{32'h1111_1111}};
		wmem[2] = {4{32'h2222_2222}};
		r = 0;
		edges = 0;
		#1;

		// Reads: the upper half of the select lies past element 1. A dynamic part-select is
		// four-valued, also in two-value logic.
		$vogls_assert_eq(mem[$vogls_blackbox(1)][$vogls_blackbox(12) +: 8] === 8'bxxxx_0010, 1'b1);
		$vogls_assert_eq(
			wmem[$vogls_blackbox(1)][$vogls_blackbox(96) +: 64]
				=== {32'bx, 32'h1111_1111},
			1'b1);
		// In-bounds selects are unaffected.
		$vogls_assert_eq(mem[$vogls_blackbox(2)][$vogls_blackbox(4) +: 8], 8'h33);
		$vogls_assert_eq(wmem[$vogls_blackbox(2)][$vogls_blackbox(32) +: 64], 64'h2222_2222_2222_2222);

		// Writes: only the part inside element 1 is written.
		mem[$vogls_blackbox(1)][$vogls_blackbox(12) +: 8] = 8'hff;
		$vogls_assert_eq(mem[1], 16'hf222);
		$vogls_assert_eq(mem[2], 16'h3333);
		wmem[$vogls_blackbox(1)][$vogls_blackbox(96) +: 64] = 64'hffff_ffff_ffff_ffff;
		$vogls_assert_eq(wmem[1], {32'hffff_ffff, {3{32'h1111_1111}}});
		$vogls_assert_eq(wmem[2], {4{32'h2222_2222}});

		// A dynamic bit write wakes an edge watcher on the written bit, and an out-of-range one
		// writes nothing.
		r[$vogls_blackbox(1)] = 1'b1;
		#1;
		$vogls_assert_eq(edges, 0);
		r[$vogls_blackbox(0)] = 1'b1;
		#1;
		$vogls_assert_eq(edges, 1);
	end
endmodule
