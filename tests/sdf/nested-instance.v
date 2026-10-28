// vogls: verify-stdout
// vogls: annotate-sdf
`timescale 1ns/1ns
module buffer(input a, output b);
    assign b = a;
endmodule
module wrap(input a, output b);
    buffer u(.a(a), .b(b));
endmodule
module top();
    reg a;
    wire b;

    wrap w(.a(a), .b(b));

	initial #9 forever @(b) $display("[T=%0dns] b = %b", $time, b);
    initial begin
		#0   a = 0;
        #10; a = 1;
    end
endmodule
