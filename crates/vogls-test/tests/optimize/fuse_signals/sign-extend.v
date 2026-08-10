module x();
   reg signed [15:0] s;
   wire signed [17:0] b = s;
   initial begin
      s = -1; #0 $vogls_assert_eq(b, -1);
   end
endmodule
