// A narrower unsigned value is zero-extended into a wider net, both for assignments and ports.
module sub(input [7:0] i, output [3:0] o);
    assign o = i[3:0];
endmodule

module top;
    reg [3:0] b;
    reg [7:0] r;

    wire [7:0] assigned;
    assign assigned = b;

    wire [7:0] declared = b;

    wire [7:0] wide_out;
    wire [3:0] narrow_out;
    sub s0 (.i(b), .o(narrow_out));
    sub s1 (.i(r), .o(wide_out));

    initial begin
        b = 4'h5;
        r = 8'hA6;
        #1;
        $vogls_assert_eq(assigned, 8'h05);
        $vogls_assert_eq(declared, 8'h05);
        $vogls_assert_eq(narrow_out, 4'h5);
        $vogls_assert_eq(wide_out, 8'h06);
        b = 4'hF;
        #1;
        $vogls_assert_eq(assigned, 8'h0F);
        $vogls_assert_eq(declared, 8'h0F);
    end
endmodule
