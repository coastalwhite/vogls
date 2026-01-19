module top_module ( input in, output out );
    assign out = in;
endmodule

module tb();
    reg in;
    wire out;

    top_module i(in, out);

    initial begin
        in = 1'b0;
        #1 $vogls_assert_eq(out, 1'b0); in = 1'b1;
        #1 $vogls_assert_eq(out, 1'b1); in = 1'b0;
        #1 $vogls_assert_eq(out, 1'b0);
    end
endmodule
