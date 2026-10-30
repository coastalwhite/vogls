// Output ports fused into slices of instance arrays, wider and narrower nets, part-selects and
// inout ports.
module inv(input [3:0] i, output [3:0] o);
    assign o = ~i;
endmodule
module src4(output [3:0] o);
    assign o = 4'hA;
endmodule
module src8(output [7:0] o);
    assign o = 8'h5C;
endmodule
module bidir(inout [3:0] p);
endmodule

module top;
    reg  [11:0] in;
    wire [11:0] out;
    inv u[2:0] (.i(in), .o(out));

    // A narrower port is zero-extended, a wider one truncated.
    wire [7:0] wide;
    src4 a (.o(wide));
    wire [3:0] narrow;
    src8 b (.o(narrow));

    wire [15:0] part;
    src4 c (.o(part[11:8]));
    assign part[7:0] = 8'h00;
    assign part[15:12] = 4'h0;

    wire [3:0] w;
    bidir d (.p(w));
    assign w = 4'h9;

    initial begin
        in = 12'h5A3;
        #1;
        $vogls_assert_eq(out, 12'hA5C);
        $vogls_assert_eq(wide, 8'h0A);
        $vogls_assert_eq(narrow, 4'hC);
        $vogls_assert_eq(part, 16'h0A00);
        $vogls_assert_eq(w, 4'h9);
        $vogls_assert_eq(d.p, 4'h9);
        in = 12'h0F0;
        #1;
        $vogls_assert_eq(out, 12'hF0F);
    end
endmodule
