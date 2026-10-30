// A connection is extended by the signedness of the value that drives it, not of what it drives.
// Fusing only zero-extends, so a narrower signed value must not be fused.
module neg(output signed [3:0] o);
    assign o = -4'sd3;
endmodule
module signed_in(input signed [7:0] i, output [7:0] o);
    assign o = i;
endmodule

module top;
    // Signed output port into a wider unsigned net: sign-extended.
    wire [7:0] a;
    neg n (.o(a));

    // Signed value into a wider signed input port: sign-extended.
    reg signed [3:0] s;
    wire [7:0] b;
    signed_in m0 (.i(s), .o(b));

    // Unsigned value into a wider signed input port: zero-extended.
    reg [3:0] u;
    wire [7:0] c;
    signed_in m1 (.i(u), .o(c));

    // Signed constants into wider nets.
    wire [7:0] d = 4'sd5;
    wire [7:0] e;
    assign e = 4'sb1101;

    initial begin
        s = -4'sd3;
        u = 4'hD;
        #1;
        $vogls_assert_eq(a, 8'hFD);
        $vogls_assert_eq(b, 8'hFD);
        $vogls_assert_eq(c, 8'h0D);
        $vogls_assert_eq(d, 8'h05);
        $vogls_assert_eq(e, 8'hFD);
    end
endmodule
