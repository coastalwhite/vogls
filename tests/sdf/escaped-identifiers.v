// vogls: verify-stdout
// vogls: annotate-sdf
`timescale 1ns/1ns
module buffer(input a, output b);
    assign b = a;
endmodule
module top();
    reg in;
    wire mid, out;

    buffer \u[0].buf  (.a(in),  .b(mid));
    buffer \u[1].buf  (.a(mid), .b(out));

	initial #9 forever @(out) $display("[T=%0dns] out = %b", $time, out);
    initial begin
		#0   in = 0;
        #10; in = 1;
    end
endmodule
