// A signed source that is narrower than its destination is sign-extended as wiring, both for a
// continuous assignment and for an output port.
module narrow(input signed [1:0] i, output signed [1:0] o);
   assign o = i;
endmodule

module tb;
   reg  signed [1:0] s;
   wire        [5:0] w;
   wire        [4:0] p;
   wire        [3:0] c;

   assign w = s;
   assign c = {s, 2'b01};
   narrow n(.i(s), .o(p));

   initial begin
      s = 2'b01;
      #1;
      $vogls_assert_eq(w, 6'b000001);
      $vogls_assert_eq(p, 5'b00001);
      $vogls_assert_eq(c, 4'b0101);

      s = 2'b10;
      #1;
      $vogls_assert_eq(w, 6'b111110);
      $vogls_assert_eq(p, 5'b11110);
      $vogls_assert_eq(c, 4'b1001);
   end
endmodule
