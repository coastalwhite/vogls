// A drive whose position is only known when it runs: what it writes, what it leaves alone, and
// whether it counts as a change, which is what wakes a watcher. Writes at one time wake it once.
//
// The index may point anywhere. Bits that would land outside the signal are dropped, whether the
// index runs past the top or is negative (and so wraps around to a huge one) -- and a write that
// drops every bit is no change at all. An index holding x is `drive-slice-x-index.v`.
`timescale 1fs / 1fs
module top();
    reg [7:0] narrow;
    reg [63:0] word;
    reg [99:0] wide;
    reg [127:0] mem [0:3];
    integer i, wakes;

    always @(narrow or word or wide) wakes = wakes + 1;

    initial begin
        wakes = 0;
        narrow = 8'h00;
        word = 64'h0;
        wide = 100'h0;
        #1 $vogls_assert_eq(wakes, 1);

        // In range, narrow signal.
        i = 2;
        narrow[i +: 4] = 4'hf;
        $vogls_assert_eq(narrow, 8'h3c);
        narrow[i +: 4] = 4'h5;
        $vogls_assert_eq(narrow, 8'h14);
        #1 $vogls_assert_eq(wakes, 2);

        // Running past the top: only the bits inside the signal move.
        i = 6;
        narrow[i +: 4] = 4'hf;
        $vogls_assert_eq(narrow, 8'hd4);
        #1 $vogls_assert_eq(wakes, 3);

        // A negative index lands nowhere, and wakes nothing.
        i = -1;
        narrow[i +: 4] = 4'hf;
        $vogls_assert_eq(narrow, 8'hd4);
        word[i] = 1'b1;
        $vogls_assert_eq(word, 64'h0);
        word[i +: 8] = 8'hff;
        $vogls_assert_eq(word, 64'h0);
        #1 $vogls_assert_eq(wakes, 3);

        // Far past the top of a word-sized signal.
        i = 100;
        word[i +: 8] = 8'hff;
        $vogls_assert_eq(word, 64'h0);
        #1 $vogls_assert_eq(wakes, 3);

        // Straddling the top of a word-sized signal.
        i = 60;
        word[i +: 8] = 8'hff;
        $vogls_assert_eq(word, 64'hf000_0000_0000_0000);
        #1 $vogls_assert_eq(wakes, 4);

        // A wide signal, at a position that crosses one of its words.
        i = 60;
        wide[i +: 8] = 8'hff;
        $vogls_assert_eq(wide, 100'hff << 60);
        wide[i +: 8] = 8'h81;
        $vogls_assert_eq(wide, 100'h81 << 60);
        #1 $vogls_assert_eq(wakes, 5);

        // Writing the same value again is no change.
        wide[i +: 8] = 8'h81;
        #1 $vogls_assert_eq(wakes, 5);

        // Straddling the top of a wide signal, and beyond it on either side.
        i = 96;
        wide[i +: 8] = 8'hff;
        $vogls_assert_eq(wide, (100'h81 << 60) | (100'hf << 96));
        #1 $vogls_assert_eq(wakes, 6);
        i = 200;
        wide[i +: 8] = 8'hff;
        i = -4;
        wide[i +: 8] = 8'hff;
        $vogls_assert_eq(wide, (100'h81 << 60) | (100'hf << 96));
        #1 $vogls_assert_eq(wakes, 6);

        // A value wider than a word, written at a position that is not word aligned.
        i = 5;
        wide[i +: 96] = {96{1'b1}};
        $vogls_assert_eq(wide, {{95{1'b1}}, 5'b0});
        i = 6;
        wide[i +: 65] = 65'h1_0000_0000_0000_0001;
        $vogls_assert_eq(wide, {{29{1'b1}}, 1'b1, 63'b0, 1'b1, 1'b1, 5'b0});
        #1 $vogls_assert_eq(wakes, 7);

        // An array element written through a computed index.
        mem[0] = 128'h0; mem[1] = 128'h0; mem[2] = 128'h0; mem[3] = 128'h0;
        i = 2;
        mem[i] = 128'h1234;
        mem[i][127:120] = 8'hab;
        $vogls_assert_eq(mem[2], 128'hab00_0000_0000_0000_0000_0000_0000_1234);
        $vogls_assert_eq(mem[1], 128'h0);
        $vogls_assert_eq(mem[3], 128'h0);

    end
endmodule
