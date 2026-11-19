// An assignment that only wires bits of a net into other bits of the same net, here a shift
// through the net itself, propagates through the entire net.
module tb;
   reg        r;
   wire [3:0] w;
   assign w = {w[2:0], r};

   initial begin
      r = 1'b1;
      #1;
      $vogls_assert_eq(w, 4'b1111);
      r = 1'b0;
      #1;
      $vogls_assert_eq(w, 4'b0000);
   end
endmodule
