module top_module (
    input [7:0] a, b, c, d,
    output [7:0] min
);
    wire [7:0] min_a_b, min_c_d;

    assign min_a_b = (a < b) ? a : b;
    assign min_c_d = (c < d) ? c : d;
    assign min = (min_a_b < min_c_d) ? min_a_b : min_c_d;
endmodule

module tb();
    reg [7:0] a, b, c, d;
    wire [7:0] min;

    top_module m(a, b, c, d, min);

    initial begin
        #1 a = 0; b = 0; c = 0; d = 0;
        #1 $vogls_assert_eq(min, 0); a = 10; 
        #1 $vogls_assert_eq(min, 0); b = 17;
        #1 $vogls_assert_eq(min, 0); c = 32;
        #1 $vogls_assert_eq(min, 0); d = 7;
        #1 $vogls_assert_eq(min, 7); b = 3;
        #1 $vogls_assert_eq(min, 3); a = 6;
        #1 $vogls_assert_eq(min, 3); b = 9;
        #1 $vogls_assert_eq(min, 6); c = 2;
        #1 $vogls_assert_eq(min, 2);
    end
endmodule
