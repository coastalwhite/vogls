module top_module ( output out );
    assign out = 1'b0;
endmodule

module tb();
    wire out;

    top_module i(out);

    initial begin
        #1 $vogls_assert_eq(out, 1'b0);
        #1 $vogls_assert_eq(out, 1'b0);
    end
endmodule
