// A narrower initialized value fused into a wider port gets inverted into a slice of the port.
// Its initial value has to move along.
module sub(input [7:0] i, output [7:0] o);
    assign o = i;
endmodule
module signed_in(input signed [7:0] i, output [7:0] o);
    assign o = i;
endmodule

module top;
    // A narrower initialized value into a wider port.
    reg [3:0] b = 4'h5;
    wire [7:0] o0;
    sub s0 (.i(b), .o(o0));

    // The same into a wider signed port.
    reg [3:0] u = 4'hD;
    wire [7:0] o1;
    signed_in s1 (.i(u), .o(o1));

    initial begin
        #1;
        $vogls_assert_eq(o0, 8'h05);
        $vogls_assert_eq(o1, 8'h0D);
    end
endmodule
