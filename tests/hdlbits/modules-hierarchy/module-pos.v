module mod_a ( output out1, output out2, input in1, input in2, input in3, input in4 );
    assign out1 = in1 ^ in2;
    assign out2 = in3 ^ in4;
endmodule

module top_module ( input a, input b, input c, input d, output out1, output out2 );
    mod_a i (out1, out2, a, b, c, d);
endmodule

module tb();
    reg a, b, c, d;
    wire out1, out2;

    top_module m(a, b, c, d, out1, out2);

    initial begin
        #1 a = 0; b = 0; c = 0; d = 0; #1
        $vogls_assert_eq(out1, 0);
        $vogls_assert_eq(out2, 0);

        #1 a = 1; #1
        $vogls_assert_eq(out1, 1);
        $vogls_assert_eq(out2, 0);

        #1 c = 1; #1
        $vogls_assert_eq(out1, 1);
        $vogls_assert_eq(out2, 1);

        #1 b = 1; #1
        $vogls_assert_eq(out1, 0);
        $vogls_assert_eq(out2, 1);

        #1 d = 1; #1
        $vogls_assert_eq(out1, 0);
        $vogls_assert_eq(out2, 0);
    end
endmodule
