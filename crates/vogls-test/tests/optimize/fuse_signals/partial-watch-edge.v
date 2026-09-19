// A watched signal folded into one bit of a wider one.
//
// `s` has a single partial driver, so it is fused into `w[2]` rather than kept as a signal of its
// own, and the watch on it becomes a watch on that one bit. Writes to `w` are four bits wide, so
// they reach the watch whenever *any* of those bits moved -- the edge has to be decided from bit 2
// alone, both when it holds low and when it holds high across such a write.
`timescale 1fs / 1fs
module top();
    reg [3:0] w;
    wire s;
    assign s = w[2];

    integer posedges;

    always @(posedge s) posedges = posedges + 1;

    initial begin
        posedges = 0;
        w = 4'b0000;

        // A neighbouring bit rises while `s` holds low.
        #1 w = 4'b0001;
        #1 $vogls_assert_eq(posedges, 0);

        // `s` rises.
        #1 w = 4'b0100;
        #1 $vogls_assert_eq(posedges, 1);

        // Neighbouring bits move while `s` holds high.
        #1 w = 4'b0101;
        #1 $vogls_assert_eq(posedges, 1);
        #1 w = 4'b0111;
        #1 $vogls_assert_eq(posedges, 1);

        // `s` falls, and a neighbour moves while it holds low again.
        #1 w = 4'b0011;
        #1 $vogls_assert_eq(posedges, 1);
        #1 w = 4'b0010;
        #1 $vogls_assert_eq(posedges, 1);

        // `s` rises once more.
        #1 w = 4'b0110;
        #1 $vogls_assert_eq(posedges, 2);
    end
endmodule
