module tb;
   reg [63:0]  ok64    [0:65534];
   reg [63:0]  bad64   [0:65535];
   reg [511:0] okwide  [0:8190];
   reg [511:0] badwide [0:8191];
   reg [511:0] partial [0:8999];
   initial begin
      ok64[1]     = 64'h11;
      bad64[1]    = 64'h22;
      okwide[1]   = 512'h33;
      badwide[1]  = 512'h44;
      partial[1]   = 512'h55;
      partial[800] = 512'h66;
      partial[900] = 512'h77;
      #1;
      $vogls_assert_eq(ok64[1], 64'h11);
      $vogls_assert_eq(bad64[1], 64'h22);
      $vogls_assert_eq(okwide[1], 64'h33);
      $vogls_assert_eq(badwide[1], 64'h44);
      $vogls_assert_eq(partial[1], 64'h55);
      $vogls_assert_eq(partial[800], 64'h66);
      $vogls_assert_eq(partial[900], 64'h77);
   end
endmodule
