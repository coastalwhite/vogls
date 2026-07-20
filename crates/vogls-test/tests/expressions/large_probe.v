module tb;
   reg [63:0] m [0:262143];
   initial begin
      m[5] = 64'hAB;
      $vogls_assert_eq(m[5], 64'hAB);
   end
endmodule
