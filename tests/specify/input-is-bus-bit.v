// The input of a module path can be aliased to a bit of a wider net, while the process of the path
// takes its last update time.
`timescale 1ns/1ns
module buf1(input a, output y);
    assign y = a;
    specify (a => y) = 1; endspecify
endmodule

module top;
    reg [1:0] r;
    wire [1:0] w;
    wire y;
    assign w = r;
    buf1 u (w[1], y);

    initial begin
        r = 0;
        #5 r = 2'b10;
        #2 $vogls_assert_eq(y, 1'b1);
        $finish;
    end
endmodule
