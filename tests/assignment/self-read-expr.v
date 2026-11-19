// An assignment whose expressions read bits that the same assignment drives must still see its own
// updates.
module tb;
   reg        a, b;
   wire [2:0] w;

   assign w[0]   = a;
   assign w[2:1] = {w[0] ^ b, w[2] ^ w[0]};

   initial begin
      a = 1'b0; b = 1'b0;
      #1;
      $vogls_assert_eq(w, 3'b000);

      b = 1'b1;
      #1;
      $vogls_assert_eq(w, 3'b110);

      a = 1'b1;
      #1;
      $vogls_assert_eq(w, 3'b011);
   end
endmodule
