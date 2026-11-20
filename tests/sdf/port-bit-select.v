// vogls: annotate-sdf
//
// SDF ports can select bits of a vector net, like the pads of a padring that connect to the bits of
// the ports of the top-level module. A zero delay between them adds no delay, while the rest of the
// file is still annotated.
`timescale 1ns/1ns
module buffer(input a, output b);
    assign b = a;
endmodule

module top;
    reg [3:0] in;
    wire [3:0] out;
    wire [0:3] rev;

    buffer u0 (.a(in[0]), .b(out[0]));
    buffer u1 (.a(in[1]), .b(out[1]));
    assign rev = in;
    buffer u2 (.a(rev[1]), .b(out[2]));
    assign out[3] = in[3];

    initial begin
        in = 0;
        #10;
        $vogls_assert_eq(out, 4'b0000);

        in = 4'b1111;
        #1;
        $vogls_assert_eq(out, 4'b1101);
        #2;
        $vogls_assert_eq(out, 4'b1111);
        $finish;
    end
endmodule
