// vogls: mode=four-value-logic
// vogls: verify-stdout[sort-lines]
`timescale 1ns/1ns
module tb();
    reg a, en;
    reg [3:0] b;
    wire s;
    wire [3:0] v, w;

    assign #(1, 2, 3) s = en ? a : 1'bz;
    assign #(1, 2, 3) v = en ? b : 4'bz;
    assign #4 w = b;

    // Skip the outputs settling at the start of the simulation.
    always @(s) if ($time >= 10) $display("[T=%0t] s = %b", $time, s);
    always @(v) if ($time >= 10) $display("[T=%0t] v = %b", $time, v);
    always @(w) if ($time >= 10) $display("[T=%0t] w = %b", $time, w);

    initial begin
        a = 0; en = 1; b = 3;
        #10 begin a = 1; b = 0; end
        #10 b = 5;
        #10 b = 6;
        #10 en = 0;
        #10 en = 1;
        #10 begin a = 1'bx; b = 0; end
        #10 b = 7;      // a pulse shorter than the delay of `w`
        #2  b = 0;
    end
endmodule
