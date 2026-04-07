module top_module(
    input clk,
    input areset,
    input in,
    output out);

    parameter A=1'b0, B=1'b1; 
    reg state, next_state;

    always @(*) begin
		next_state = state;
		case (state)
			A: if (in) next_state = A;
			   else    next_state = B;
			B: if (in) next_state = B;
			   else    next_state = A;
		endcase
    end

    always @(posedge clk, posedge areset) begin
		if (areset) state <= A;
		else        state <= next_state;
    end

    assign out = state;
endmodule

module tb();
	reg clk, areset, in;
	wire out;

	top_module t(clk, areset, in, out);

	always begin clk = 0; #5 clk = 1; #5 ; end

	initial begin
		#1
		in = 1;
		areset = 1;
		#0
		areset = 0;
		$vogls_assert_eq(out, t.A);
		#10
		$vogls_assert_eq(out, t.A);
		in = 0;
		#10
		$vogls_assert_eq(out, t.B);
		#10
		$vogls_assert_eq(out, t.A);
        $finish();
	end
endmodule
