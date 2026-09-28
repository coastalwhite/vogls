module tb;
   reg signed [7:0]  m  [0:1];
   initial begin
      m[0] = -8'sd12; m[1] = 8'sd4;

      $vogls_assert_eq(((((m[0]) & 0) - 1) < 0), 1'b1);
      $vogls_assert_eq((((($unsigned(m[0])) & 0) - 1) < 0), 1'b0);
   end
endmodule
