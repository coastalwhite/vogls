module tb;
   localparam N = 4;

   wire [N-1:0] hit;
   genvar j;
   generate
      for (j = N - 1; j >= 0; j = j - 1) begin : st
         assign hit[j] = 1'b1;
      end
   endgenerate

   initial begin
      #1;
      $vogls_assert_eq(hit, {N{1'b1}});
   end
endmodule
