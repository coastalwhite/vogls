// The module paths of an output can be spread over several specify blocks.
`timescale 1ns/1ns
module and2(input a, input b, output y);
    assign y = a & b;
    specify
        (a => y) = 2;
    endspecify
    specify
        (b => y) = 4;
    endspecify
endmodule

module top();
    reg a, b;
    wire y;
    and2 u (a, b, y);

    initial begin
        a = 1; b = 1;
        #10;
        $vogls_assert_eq(y, 1'b1);

        a = 0;
        #1;
        $vogls_assert_eq(y, 1'b1);
        #2;
        $vogls_assert_eq(y, 1'b0);

        a = 1;
        #10;
        b = 0;
        #3;
        $vogls_assert_eq(y, 1'b1);
        #2;
        $vogls_assert_eq(y, 1'b0);
        $finish;
    end
endmodule
