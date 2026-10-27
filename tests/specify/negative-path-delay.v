// vogls: mode=four-value-logic
// vogls: verify-stdout[sort-lines]
//
// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 222
// "If the path delay expression results in a negative value, it shall be treated as zero."
`timescale 1ns/1ns
module gate(
    input i,
    output o_neg, o_x
);
    assign o_neg = i, o_x = i;

    specify
        (i => o_neg) = (-2, 3);
        (i => o_x)   = (1'bx, 4);
    endspecify
endmodule

module tb();
    reg i;
    wire o_neg, o_x;

    gate g(i, o_neg, o_x);

    always @(o_neg) $display("[T=%0t] o_neg = %b", $time, o_neg);
    always @(o_x)   $display("[T=%0t] o_x = %b", $time, o_x);

    initial begin
        #10 i = 1'b1;   // rise: the negative and x delays are zero
        #10 i = 1'b0;   // fall: 3 and 4
    end
endmodule
