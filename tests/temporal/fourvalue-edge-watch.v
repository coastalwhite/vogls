// vogls: mode=four-value-logic
// `posedge` on a four-valued signal is not just "settles at 1": it also fires on a transition
// through x or z, and a move between x and z is no edge at all. The watch decides this from the
// value planes before and after rather than from the level it ends on, so all of it has to hold
// with the software edge check gone.
`timescale 1fs / 1fs
module top();
    reg s;

    integer posedges;
    integer negedges;

    always @(posedge s) posedges = posedges + 1;
    always @(negedge s) negedges = negedges + 1;

    initial begin
        posedges = 0;
        negedges = 0;
        s = 1'b0;

        // Leaving a known 0 counts as a rise, whatever it leaves for.
        #1 s = 1'bx;
        #1 $vogls_assert_eq(posedges, 1);
        #0 $vogls_assert_eq(negedges, 0);

        // Between the two unknowns there is no edge either way.
        #1 s = 1'bz;
        #1 $vogls_assert_eq(posedges, 1);
        #0 $vogls_assert_eq(negedges, 0);

        // Arriving at a known 1 is a rise.
        #1 s = 1'b1;
        #1 $vogls_assert_eq(posedges, 2);
        #0 $vogls_assert_eq(negedges, 0);

        // Leaving a known 1 is a fall, and arriving back at a known 0 is another.
        #1 s = 1'bx;
        #1 $vogls_assert_eq(posedges, 2);
        #0 $vogls_assert_eq(negedges, 1);
        #1 s = 1'b0;
        #1 $vogls_assert_eq(posedges, 2);
        #0 $vogls_assert_eq(negedges, 2);

        // And the ordinary case still works.
        #1 s = 1'b1;
        #1 $vogls_assert_eq(posedges, 3);
        #0 $vogls_assert_eq(negedges, 2);
    end
endmodule
