module test;
   reg [7:0] val;
   initial begin
      val = $random;
      $vogls_assert_eq(val, 36);
   end
endmodule
