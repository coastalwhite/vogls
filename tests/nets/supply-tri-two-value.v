// vogls: mode=two-value-logic
//
// In two-valued logic, the undriven bits of a `tri1` are `1` and `supply` nets keep their value
// whatever drives them.
`timescale 1ns/1ns
module tb();
    reg a, b;

    supply0 s0;
    supply1 s1;
    assign s0 = a;
    assign s1 = a;

    tri0 t0;
    tri1 t1;
    assign t0 = a;
    assign t1 = a;

    tri0 [1:0] t0_undriven;
    tri1 [1:0] t1_undriven;

    tri1 [3:0] t1_vec;
    assign t1_vec[1:0] = {a, b};

    integer i;
    initial begin
        for (i = 0; i < 4; i = i + 1) begin
            {a, b} = i;
            #1;
            $vogls_assert_eq(s0, 1'b0);
            $vogls_assert_eq(s1, 1'b1);
            $vogls_assert_eq(t0, a);
            $vogls_assert_eq(t1, a);
            $vogls_assert_eq(t0_undriven, 2'b00);
            $vogls_assert_eq(t1_undriven, 2'b11);
            $vogls_assert_eq(t1_vec, {2'b11, a, b});
        end
    end
endmodule
