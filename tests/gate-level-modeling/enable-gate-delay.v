// vogls: mode=four-value-logic
// vogls: verify-stdout[sort-lines]
`timescale 1ns/1ns
module tb();
    reg a, en;
    wire o1, o2, o3;

    notif1 #5         (o1, a, en);
    bufif0 #(2, 4)    (o2, a, en);
    bufif1 #(1, 2, 3) (o3, a, en);

    always @(o1) if ($time >= 10) $display("[T=%0t] o1 = %b", $time, o1);
    always @(o2) if ($time >= 10) $display("[T=%0t] o2 = %b", $time, o2);
    always @(o3) if ($time >= 10) $display("[T=%0t] o3 = %b", $time, o3);

    initial begin
        a = 0; en = 1;
        #10 a = 1;
        #10 en = 0;
        #10 a = 0;
        #10 en = 1;
        #10 en = 1'bx;
    end
endmodule
