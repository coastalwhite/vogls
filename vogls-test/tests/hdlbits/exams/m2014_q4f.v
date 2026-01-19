module top_module ( input in1, input in2, output out );
    assign out = in1 & ~in2;
endmodule

module tb();
    reg in1, in2;
    wire out;

    top_module i(in1, in2, out);

    initial begin
        in1 = 0; in2 = 0;
        #1 $vogls_assert_eq(out, 1'b0); in1 = 0; in2 = 1;
        #1 $vogls_assert_eq(out, 1'b0); in1 = 1; in2 = 1;
        #1 $vogls_assert_eq(out, 1'b0); in1 = 1; in2 = 0;
        #1 $vogls_assert_eq(out, 1'b1);
    end
endmodule
