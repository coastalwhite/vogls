// vogls: verify-stdout[sort-lines]
`timescale 1ps / 1ps
module gate(
    input i, j,
    output o1, o2, o3
);
    assign o1 = i;
    assign o2 = i | j;
    assign o3 = j;

    specify
        (i => o1) = 1;
        (i => o2) = 2;
        (j => o2) = 3;
        (j => o3) = 4;
    endspecify
endmodule

`timescale 1fs / 1fs
module tb();
    reg i, j;
    wire o1, o2, o3;

    gate g(i, j, o1, o2, o3);

	initial #1 forever @(o1) $display("[T=%0d] o1 = %0x", $time(), o1);
	initial #1 forever @(o2) $display("[T=%0d] o2 = %0x", $time(), o2);
	initial #1 forever @(o3) $display("[T=%0d] o3 = %0x", $time(), o3);

    initial begin
        #10_000 i = 1; j = 1;
        #10_000 j = 0;
        #10_000 i = 0;
        #10_000 j = 1;
        #10_000 ;
    end
endmodule

