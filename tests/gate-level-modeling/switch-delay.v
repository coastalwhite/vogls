// vogls: mode=four-value-logic
// vogls: verify-stdout[sort-lines]
`timescale 1ns/1ns
module tb();
    reg a, c, cn, cp;
    wire o1, o2, o3, o4;

    nmos  #(1, 2, 3) (o1, a, c);
    pmos  #4         (o2, a, c);
    rnmos #(2, 3)    (o3, a, c);
    cmos  #(1, 2, 3) (o4, a, cn, cp);

    always @(o1) if ($time >= 10) $display("[T=%0t] o1 = %b", $time, o1);
    always @(o2) if ($time >= 10) $display("[T=%0t] o2 = %b", $time, o2);
    always @(o3) if ($time >= 10) $display("[T=%0t] o3 = %b", $time, o3);
    always @(o4) if ($time >= 10) $display("[T=%0t] o4 = %b", $time, o4);

    initial begin
        a = 0; c = 1; cn = 1; cp = 0;
        #10 a = 1;
        #10 c = 0;
        #10 a = 0;
        #10 begin cn = 0; cp = 1; end
        #10 begin c = 1'bx; cn = 1'bx; end
    end
endmodule
