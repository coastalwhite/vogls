// vogls: skip=two-value-logic
// vogls: verify-stdout[sort-lines]
module gate(
    input i,
    output o1, o2, o3, o6, o12
);
    assign o1 = i, o2 = i, o3 = i, o6 = i, o12 = i;

    specify
        (i => o1)  = 1;
        (i => o2)  = 1, 2;
        (i => o3)  = 1, 2, 3;
        (i => o6)  = 1, 2, 3, 4, 5, 6;
        (i => o12) = 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12;
    endspecify
endmodule

module tb();
    reg i;
    wire o1, o2, o3, o6, o12;

    gate g(i, o1, o2, o3, o6, o12);

    always @(o1)  $display("[T=%03d] o1 = %0x",  $time(), o1);
    always @(o2)  $display("[T=%03d] o2 = %0x",  $time(), o2);
    always @(o3)  $display("[T=%03d] o3 = %0x",  $time(), o3);
    always @(o6)  $display("[T=%03d] o6 = %0x",  $time(), o6);
    always @(o12) $display("[T=%03d] o12 = %0x", $time(), o12);

    initial begin
        #20 i = 1'b0; // x -> 0 | 1 | 2 | 2 | max(tz0, t10) = max(6, 2) = 6 | 10 | 
        #20 i = 1'b1; // 0 -> 1 | 1 | 1 | 1 |                             1 |  1 | 
        #20 i = 1'bz; // 1 -> z | 1 | 2 | 3 |                             5 |  5 | 
        #20 i = 1'bx; // z -> x | 1 | 1 | 1 | min(tz1, tz0) = min(4, 6) = 4 | 12 | 
        #20 i = 1'b1; // x -> 1 | 1 | 1 | 1 | max(tz1, t01) = max(4, 1) = 4 |  8 | 
        #20 i = 1'bx; // 1 -> x | 1 | 2 | 2 | min(t1z, t10) = min(5, 2) = 2 |  9 | 
        #20 i = 1'bz; // x -> z | 1 | 2 | 3 | max(t1z, t0z) = max(5, 3) = 5 | 11 | 
        #20 i = 1'b0; // z -> 0 | 1 | 2 | 2 |                             6 |  6 | 
        #20 i = 1'bz; // 0 -> z | 1 | 1 | 3 |                             3 |  3 | 
        #20 i = 1'b1; // z -> 1 | 1 | 1 | 1 |                             4 |  4 | 
        #20 i = 1'b0; // 1 -> 0 | 1 | 2 | 2 |                             2 |  2 | 
        #20 i = 1'bx; // 0 -> x | 1 | 1 | 1 | min(t0z, t01) = min(3, 1) = 1 |  7 | 
    end
endmodule
