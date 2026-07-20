module tb();
   reg signed [7:0] x;
   initial begin
       x = 8'hAB;
       $vogls_assert_eq(x >> 4, 8'hA);
   end
endmodule
