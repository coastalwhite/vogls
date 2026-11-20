// vogls: annotate-sdf
//
// An empty value `()` in an SDF delay leaves the delay of its transitions as the module path gives
// it: here the rise delay stays 3 and the fall delay becomes 1.
`timescale 1ns/1ns
module buffer(input a, output y);
    assign y = a;
    specify
        (a => y) = (3, 5);
    endspecify
endmodule

module top;
    reg a;
    wire y;
    buffer u (a, y);

    initial begin
        a = 0;
        #10;
        $vogls_assert_eq(y, 1'b0);

        a = 1;
        #2 $vogls_assert_eq(y, 1'b0);
        #2 $vogls_assert_eq(y, 1'b1);

        #6;
        a = 0;
        #2 $vogls_assert_eq(y, 1'b0);
        $finish;
    end
endmodule
