// vogls: verify-stdout[sort-lines]
//
// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) 9.7.1
//
// """
// If the delay expression evaluates to an unknown or high-impedance value, it shall be interpreted
// as zero delay. If the delay expression evaluates to a negative value, it shall be interpreted as
// a twos-complement unsigned integer of the same size as a time variable.
// """
`timescale 1ns/1ns
module tb();
    parameter signed [7:0]  NEG8  = -2;
    parameter signed [63:0] NEG64 = -3;
    parameter [3:0]         X     = 4'b1x01;
    parameter [3:0]         Z     = 4'bzzzz;

    reg signed [7:0] v;
    integer i;
    reg [7:0] vx;
    real r;

    // Constant delays.
    initial #NEG8  $display("[T=%0t] #NEG8 = -2", $time);
    initial #NEG64 $display("[T=%0t] #NEG64 = -3", $time);
    initial #X     $display("[T=%0t] #X is a zero delay", $time);
    initial #Z     $display("[T=%0t] #Z is a zero delay", $time);

    // Non-constant delays.
    initial begin v = -5;   #(v)  $display("[T=%0t] #(v) = -5", $time); end
    initial begin i = -6;   #(i)  $display("[T=%0t] #(i) = -6", $time); end
    initial begin vx = 'bx; #(vx) $display("[T=%0t] #(vx) is a zero delay", $time); end
    initial begin r = -3.0; #(r)  $display("[T=%0t] #(r) = -3.0 is a zero delay", $time); end
endmodule
