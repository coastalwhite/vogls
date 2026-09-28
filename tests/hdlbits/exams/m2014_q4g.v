module top_module ( input in1, input in2, input in3, output out );
    assign out = ~(in1 ^ in2) ^ in3;
endmodule

module tb();
    reg in1, in2, in3;
    wire out;

    top_module i(in1, in2, in3, out);

    initial begin
        in1 = 0; in2 = 0; in3 = 0;
        #1 $vogls_assert_eq(out, 1'b1); in1 = 0; in2 = 0; in3 = 1;
        #1 $vogls_assert_eq(out, 1'b0); in1 = 0; in2 = 1; in3 = 0;
        #1 $vogls_assert_eq(out, 1'b0); in1 = 0; in2 = 1; in3 = 1;
        #1 $vogls_assert_eq(out, 1'b1); in1 = 1; in2 = 0; in3 = 0;
        #1 $vogls_assert_eq(out, 1'b0); in1 = 1; in2 = 0; in3 = 1;
        #1 $vogls_assert_eq(out, 1'b1); in1 = 1; in2 = 1; in3 = 0;
        #1 $vogls_assert_eq(out, 1'b1); in1 = 1; in2 = 1; in3 = 1;
        #1 $vogls_assert_eq(out, 1'b0);
    end
endmodule
