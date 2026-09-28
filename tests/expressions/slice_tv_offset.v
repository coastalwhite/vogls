module tb();
	reg [7:0] x;
	reg offset;

    initial begin
		x = 8'h00;
		offset = 1'h0;
		
		$vogls_assert_eq(x[offset === 1'h0 +: 1], 1'h0);
    end
endmodule
