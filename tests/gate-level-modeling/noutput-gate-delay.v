// vogls: mode=four-value-logic
// vogls: verify-stdout[sort-lines]
`timescale 1fs/1fs
module tb();
    reg a;
    wire o1, o2, o3;

    buf #3 (o1, a);
    not #(2, 5) (o2, a);
    buf #(4) (o3, a);

    always @(o1) $display("[T=%03d] o1 = %0x", $time(), o1);
    always @(o2) $display("[T=%03d] o2 = %0x", $time(), o2);
    always @(o3) $display("[T=%03d] o3 = %0x", $time(), o3);

    initial begin
        a = 1'b0;
        #20 a = 1'b1;
        #20 a = 1'b0;
        #20 a = 1'b1;
        #1  a = 1'b0;
        #20 a = 1'bz;
        #20 a = 1'b1;
    end
endmodule
