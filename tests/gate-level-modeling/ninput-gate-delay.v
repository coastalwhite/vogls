// vogls: mode=four-value-logic
// vogls: verify-stdout[sort-lines]
`timescale 1fs/1fs
module tb();
    reg a, b;
    wire o1, o2, o3;

    and #3 (o1, a, b);
    and #(2, 5) (o2, a, b);
    or #(4) (o3, a, b);

    always @(o1) $display("[T=%03d] o1 = %0x", $time(), o1);
    always @(o2) $display("[T=%03d] o2 = %0x", $time(), o2);
    always @(o3) $display("[T=%03d] o3 = %0x", $time(), o3);

    initial begin
        a = 1'b0; b = 1'b0;
        #20 a = 1'b1;
        #20 b = 1'b1;
        #20 b = 1'b0;
        #20 b = 1'b1;
        #1  b = 1'b0;
        #20 b = 1'bx;
        #20 b = 1'b1;
    end
endmodule
