// vogls: verify-stdout
`timescale 1ns/100ps
module tb();
    parameter real D_REAL = 1.5;
    parameter D_UNKNOWN = 4'b1x01;

    real r;

    initial begin
        #D_REAL    $display("[T=%0.2f] #D_REAL", $realtime);
        #D_UNKNOWN $display("[T=%0.2f] #D_UNKNOWN is a zero delay", $realtime);
        #(1.5)     $display("[T=%0.2f] #(1.5)", $realtime);

        r = 1.25; #(r) $display("[T=%0.2f] #(1.25)", $realtime);
        r = 0.04; #(r) $display("[T=%0.2f] #(0.04) rounds to zero", $realtime);
        r = 2.06; #(r) $display("[T=%0.2f] #(2.06) rounds to 2.1", $realtime);
    end
endmodule
