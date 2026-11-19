// Writes with a dynamic part-select wake the processes that read the written bits. Also when the
// read bits do not start at the written signal's first bit, and when consecutive writes share
// their offset.
module tb;
   reg [31:0] r, q;
   reg [7:0]  ma [0:3], mb [0:3];
   integer    j;

   wire [7:0] r0 = r[7:0], r1 = r[15:8], r3 = r[31:24];
   wire [7:0] q0 = q[7:0], q1 = q[15:8], q3 = q[31:24];
   wire [7:0] m1 = mb[1];
   wire [7:0] nr0 = ~r0, nr1 = ~r1, nr3 = ~r3;
   wire [7:0] nq0 = ~q0, nq1 = ~q1, nq3 = ~q3;
   wire [7:0] nm1 = ~m1;

   initial begin
      r = 0; q = 0;
      for (j = 0; j < 4; j = j + 1) begin
         ma[j] = j + 1;
         mb[j] = j + 5;
      end
      for (j = 0; j < 4; j = j + 1) begin
         r[j*8 +: 8] = ma[j];
         q[j*8 +: 8] = mb[j];
      end
      #1;
      $vogls_assert_eq(nr0, 8'hfe);
      $vogls_assert_eq(nr1, 8'hfd);
      $vogls_assert_eq(nr3, 8'hfb);
      $vogls_assert_eq(nq0, 8'hfa);
      $vogls_assert_eq(nq1, 8'hf9);
      $vogls_assert_eq(nq3, 8'hf7);
      $vogls_assert_eq(nm1, 8'hf9);

      // Not aligned to the reads.
      j = 4;
      r[j +: 8] = 8'hff;
      q[j +: 8] = 8'h00;
      #1;
      $vogls_assert_eq(nr0, 8'h0e);
      $vogls_assert_eq(nr1, 8'hf0);
      $vogls_assert_eq(nq0, 8'hfa);
      $vogls_assert_eq(nq1, 8'hff);
   end
endmodule
