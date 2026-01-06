module mod_a ( input in1, input in2, output out );
    assign out = in1 ^ in2;
endmodule

module top_module ( input a, input b, output out );
    mod_a i (a, b, out);
endmodule

module tb();
    reg a, b;
    wire out;

    top_module m(a, b, out);

    initial begin
        #1 a = 0; b = 0;
        #1 $vogls_assert_eq(out, 0);

        #1 a = 0; b = 1;
        #1 $vogls_assert_eq(out, 1);

        #1 a = 1; b = 0;
        #1 $vogls_assert_eq(out, 1);

        #1 a = 1; b = 1;
        #1 $vogls_assert_eq(out, 0);
    end
endmodule
