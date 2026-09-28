// vogls: verify-stdout
// vogls: annotate-sdf
`timescale 1fs / 1fs
module buffer(input a, output b);
    assign b = a;
endmodule
module top();
    reg a;
    wire b;

    buffer _b(a, b);
	initial #9_000_000 forever @(b) $display("[T=%0dns] b = %b", $time / 1_000_000, b);
    initial begin
		#0          a = 0;
        #10_000_000 a = 1;
    end
endmodule
