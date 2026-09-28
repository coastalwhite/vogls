module tb;
   localparam signed [63:0] X1 = (64'sd8 >>> 3);
   localparam signed [63:0] X2 = (-64'sd8 >>> 3);

   localparam signed [63:0] Y1 = (64'sd1 <<< 3);
   localparam signed [63:0] Y2 = (-64'sd1 <<< 3);

   initial begin
       $vogls_assert_eq(X1, 64'sd1);
       $vogls_assert_eq(X2, -64'sd1);

       $vogls_assert_eq(Y1, 64'sd8);
       $vogls_assert_eq(Y2, -64'sd8);
   end
endmodule
