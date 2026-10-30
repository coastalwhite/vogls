// Subset inversion folds `b` and `c` into slices of `a`. Their initial values have to move along.
module top;
    reg [3:0] b = 4'h5;
    reg [3:0] c = 4'hA;
    wire [7:0] a;

    assign a[3:0] = b;
    assign a[7:4] = c;

    initial begin
        #1;
        $vogls_assert_eq(a, 8'hA5);
        b = 4'h1;
        #1;
        $vogls_assert_eq(a, 8'hA1);
    end
endmodule
