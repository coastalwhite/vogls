// vogls: panic
module top;
  localparam [3:0] A = 4'hf;
  localparam [7:0] P = A[0+:8];

  initial $vogls_assert_eq(P, 8'bxxxx1111);
endmodule
