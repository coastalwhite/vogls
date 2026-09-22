// The changed-bits mask a drive produces, which is what decides pokes and watch wakes.
// `$vogls_drive(sig, value)` performs the drive and evaluates to that mask.

`timescale 1fs / 1fs
module top();
    reg [7:0] narrow;
    reg [99:0] wide;

    initial begin
        narrow = 8'h00;
        // Every bit moves, then none, then only the ones that differ.
        $vogls_assert_eq($vogls_drive(narrow, 8'hFF), 8'hFF);
        $vogls_assert_eq($vogls_drive(narrow, 8'hFF), 8'h00);
        $vogls_assert_eq($vogls_drive(narrow, 8'hF0), 8'h0F);
        $vogls_assert_eq($vogls_drive(narrow, 8'h00), 8'hF0);

        // A signal wider than a machine word gets a mask just as wide, spanning its words.
        wide = 100'h0;
        $vogls_assert_eq($vogls_drive(wide, 100'h5), 100'h5);
        $vogls_assert_eq($vogls_drive(wide, 100'h5), 100'h0);
        $vogls_assert_eq($vogls_drive(wide, 100'h6), 100'h3);
        // Clear it again, then move only its top bit: the mask reaches the high word.
        $vogls_assert_eq($vogls_drive(wide, 100'h0), 100'h6);
        $vogls_assert_eq($vogls_drive(wide, {1'b1, 99'h0}), {1'b1, 99'h0});
    end
endmodule
