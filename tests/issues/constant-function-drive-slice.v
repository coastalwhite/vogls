// Bit-select write with a variable index inside a constant function.
// The constant-function evaluator hits `todo!()` on `DriveSlice`.
// An out-of-range index must leave the variable unchanged.
module top;
  function [7:0] set_bit(input integer i);
    reg [7:0] r;
    begin
      r = 8'h00;
      r[i] = 1'b1;
      set_bit = r;
    end
  endfunction

  localparam [7:0] TOP = set_bit(7);
  localparam [7:0] OOB = set_bit(9);

  initial begin
    $vogls_assert_eq(TOP, 8'h80);
    $vogls_assert_eq(OOB, 8'h00);
  end
endmodule
