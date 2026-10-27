// vogls: verify-stdout[sort-lines]
`timescale 1ns/1ns
module tb();
    parameter [63:0] D64   = 64'd3;
    parameter [79:0] FIT80 = 80'h0_0000_0000_0000_0005;
    parameter [79:0] BIG80 = 80'h1_0000_0000_0000_0005;
    parameter signed [79:0] NEG80 = -4;

    initial #D64   $display("[T=%0t] #D64 = 3", $time);
    initial #FIT80 $display("[T=%0t] #FIT80 = 5", $time);
    initial #BIG80 $display("[T=%0t] #BIG80 saturates and must not fire", $time);
    initial #NEG80 $display("[T=%0t] #NEG80 saturates and must not fire", $time);

    initial #100 $finish;
endmodule
