module top_module( input in, output out );
	assign out = ~in;
endmodule

module tb();
	reg r;
	wire out;
	top_module m(r, out);
	initial begin
		r <= 0;
		#1 $vogls_assert_eq(out, 1);
		
		r <= 1;
		#1 $vogls_assert_eq(out, 0);
		
		r <= 0;
		#1 $vogls_assert_eq(out, 1);
	end
endmodule
