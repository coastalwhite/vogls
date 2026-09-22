// vogls: mode=four-value-logic
// A drive through an index holding x writes nothing at all, and so wakes nothing.
`timescale 1fs / 1fs
module top();
    reg [7:0] narrow;
    reg [99:0] wide;
    reg [31:0] xi;
    integer wakes;

    always @(narrow or wide) wakes = wakes + 1;

    initial begin
        wakes = 0;
        narrow = 8'hd4;
        wide = 100'h81 << 60;
        #1 $vogls_assert_eq(wakes, 1);

        xi = 32'bx;
        narrow[xi +: 4] = 4'hf;
        $vogls_assert_eq(narrow, 8'hd4);
        wide[xi +: 8] = 8'hff;
        $vogls_assert_eq(wide, 100'h81 << 60);
        #1 $vogls_assert_eq(wakes, 1);

        // One x bit in an otherwise fine index is still unknown.
        xi = 32'b01x0;
        narrow[xi +: 4] = 4'hf;
        $vogls_assert_eq(narrow, 8'hd4);
        #1 $vogls_assert_eq(wakes, 1);

        // And the same index once known writes as usual.
        xi = 32'd2;
        narrow[xi +: 4] = 4'hf;
        $vogls_assert_eq(narrow, 8'hfc);
        #1 $vogls_assert_eq(wakes, 2);
    end
endmodule
