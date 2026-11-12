// vogls: fail
// An indexed part-select with width 0 is illegal (IEEE 1364-2005 5.2.1).
// It must be a diagnostic. vogls panics in `lower_addressing`
// (`VectorSize::new(0).unwrap()`). Icarus: "Indexed part widths must be
// constant and greater than zero."
module top;
  reg [7:0] r;
  initial r[0+:0] = 1'b0;
endmodule
