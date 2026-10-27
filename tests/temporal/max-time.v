// vogls: verify-stdout[sort-lines]
//
// A delay that does not fit in a time variable saturates to the largest time, and an event at
// exactly the largest time is still valid.
`timescale 1ns/1ns
module tb();
    parameter [79:0] BIG80 = 80'h1_0000_0000_0000_0005;
    parameter signed [79:0] NEG80 = -4;

    initial #BIG80 $display("[T=%0t] #BIG80 saturates", $time);
    initial #NEG80 $display("[T=%0t] #NEG80 saturates", $time);
endmodule
