`timescale 1fs / 1fs
module top();
    reg a, b;
    reg [1:0] e;

    assign e[0] = a;
    assign e[1] = b;

	initial begin
        #0
		a = 1;
		#1
		b = 1;
		#1
		$vogls_assert_eq($vogls_lupdt(a), 0);
		$vogls_assert_eq($vogls_lupdt(b), 1);
	end
endmodule
