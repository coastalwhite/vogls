module top_module( 
    input [3:0] in,
    output out_and,
    output out_or,
    output out_xor
);
    and(out_and, in[0], in[1], in[2], in[3]);
    or(out_or, in[0], in[1], in[2], in[3]);
    xor(out_xor, in[0], in[1], in[2], in[3]);
endmodule

module tb();
    reg [3:0] in;
    wire out_and, out_or, out_xor;

    top_module m(in, out_and, out_or, out_xor);

    initial begin
        #1 in = 4'b0000;
        #1
        $vogls_assert_eq(out_and, 0);
        $vogls_assert_eq(out_or, 0);
        $vogls_assert_eq(out_xor, 0);

        #1 in = 4'b1111;
        #1
        $vogls_assert_eq(out_and, 1);
        $vogls_assert_eq(out_or, 1);
        $vogls_assert_eq(out_xor, 0);

        #1 in = 4'b1101;
        #1
        $vogls_assert_eq(out_and, 0);
        $vogls_assert_eq(out_or, 1);
        $vogls_assert_eq(out_xor, 1);

        #1 in = 4'b0010;
        #1
        $vogls_assert_eq(out_and, 0);
        $vogls_assert_eq(out_or, 1);
        $vogls_assert_eq(out_xor, 1);

        #1 in = 4'b1001;
        #1
        $vogls_assert_eq(out_and, 0);
        $vogls_assert_eq(out_or, 1);
        $vogls_assert_eq(out_xor, 0);
    end
endmodule
