module tb;
	function [7:0] write_at(input integer i);
		reg [7:0] r;
		begin
			r = 8'h00;
			r[i +: 8] = 8'hff;
			write_at = r;
		end
	endfunction

	function [7:0] read_at(input integer i);
		reg [7:0] r;
		begin
			r = 8'ha5;
			read_at = r[i +: 8];
		end
	endfunction

	localparam [7:0] W_PARTIAL = write_at(4);
	localparam [7:0] W_OUTSIDE = write_at(8);
	localparam [7:0] W_X = write_at(32'bx);
	localparam [7:0] R_PARTIAL = read_at(4);
	localparam [7:0] R_OUTSIDE = read_at(8);
	localparam [7:0] R_X = read_at(32'bx);

	initial begin
		$vogls_assert_eq(W_PARTIAL, 8'hf0);
		$vogls_assert_eq(W_OUTSIDE, 8'h00);
		$vogls_assert_eq(W_X, 8'h00);
		$vogls_assert_eq(R_PARTIAL === 8'bxxxx_1010 || R_PARTIAL === 8'h0a, 1'b1);
		$vogls_assert_eq(R_OUTSIDE === 8'bx || R_OUTSIDE === 8'h00, 1'b1);
		$vogls_assert_eq(R_X === 8'bx || R_X === 8'h00, 1'b1);
	end
endmodule
