// A write into the middle of a variable in a constant function.
// `Bits::set_slice` takes every bit from `offset` to the top out of `src`,
// zero-extending `src`, so the result grows to 12 bits instead of 8.
// (Hidden behind the `DriveSlice` todo in constant-function-drive-slice.v.)
module top;
  function [7:0] clear_mid(input integer unused);
    reg [7:0] r;
    begin
      r = 8'hff;
      r[3:2] = 2'b00;
      clear_mid = r;
    end
  endfunction

  localparam [7:0] P = clear_mid(0);

  initial $vogls_assert_eq(P, 8'hf3);
endmodule
