// vogls: mode=four-value-logic
// vogls: verify-stdout[sort-lines]
`timescale 1ns/100ps
module tb();
    reg a;
    wire o1, o2;

    buf #(1.5) (o1, a);
    and #(1.5, 2.25) (o2, a, a);   // 2.25 rounds to 2.3 at 100ps precision

    always @(o1) $display("[T=%0.2f] o1 = %b", $realtime, o1);
    always @(o2) $display("[T=%0.2f] o2 = %b", $realtime, o2);

    initial begin
        a = 0;
        #10 a = 1;
        #10 a = 0;
    end
endmodule
