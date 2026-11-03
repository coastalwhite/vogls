// vogls: verify-stdout[sort-lines]
`timescale 1ps / 1ps
module gate(
    input i, j,
    output o1, o2
);
    assign o1 = i;
    assign o2 = j;

    specify
        (i => o1) = 1:2:3, 7:8:9;
        (j => o2) = 4, 10:11:12;
    endspecify
endmodule

`timescale 1fs / 1fs
module tb();
    reg i, j;
    wire o1, o2;

    gate g(i, j, o1, o2);

	initial #1 forever @(o1) $display("[T=%0d] o1 = %0x", $time(), o1);
	initial #1 forever @(o2) $display("[T=%0d] o2 = %0x", $time(), o2);

    initial begin
        #10_000 i = 1; j = 1;
        #10_000 i = 0; j = 0;
        #20_000 ;
    end
endmodule
