// vogls: mode=four-value-logic
// A constant part-select wider than its whole operand. The bits outside the
// operand read as x (IEEE 1364-2005 5.2.1; matches Icarus). vogls panics in
// `tv_s_slice` (`dst_size <= src_size`). Selects that only partly run past
// the end, like A[2+:4], already work.
module top;
  localparam [3:0] A = 4'hf;
  localparam [7:0] P = A[0+:8];

  initial $vogls_assert_eq(P, 8'bxxxx1111);
endmodule
