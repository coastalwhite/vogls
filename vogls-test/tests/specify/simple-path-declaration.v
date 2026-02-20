// vogls: verify-stdout
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

module tb();
    reg i, j;
    wire o1, o2, o3;

    gate g(i, j, o1, o2, o3);

    always @(o1) $display("[T=%0d] o1 = %0x", $time(), o1);
    always @(o2) $display("[T=%0d] o2 = %0x", $time(), o2);
    always @(o3) $display("[T=%0d] o3 = %0x", $time(), o3);

    initial begin
        #10 i = 1; j = 1;
        #10 j = 0;
        #10 i = 0;
        #10 j = 1;
        #10 ;
    end
endmodule

