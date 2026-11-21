// vogls: annotate-sdf
// vogls: mode=four-value-logic
//
// A condition that compares at a wider width, here 32 bits against the decimal `1`, matches the
// SDF condition at the width of the net. A condition that can never hold, `c == 2` for a single
// bit, matches no SDF condition.
`timescale 1ns/1ns
module g(input a, input c, output y);
    assign y = a ^ c;
    specify
        if (c == 1) (a => y) = 1;
        ifnone (a => y) = 1;
    endspecify
endmodule

module h(input a, input c, output y);
    assign y = a ^ c;
    specify
        if (c == 2) (a => y) = 1;
        ifnone (a => y) = 1;
    endspecify
endmodule

module top;
    reg a, c;
    wire y, z;
    g u (a, c, y);
    h v (a, c, z);

    initial begin
        a = 0; c = 1;
        #10;
        $vogls_assert_eq(y, 1'b1);
        $vogls_assert_eq(z, 1'b1);

        a = 1;
        #3;
        // The COND of `u` applies, the one of `v` does not.
        $vogls_assert_eq(y, 1'b1);
        $vogls_assert_eq(z, 1'b0);
        #3;
        $vogls_assert_eq(y, 1'b0);
        $finish;
    end
endmodule
