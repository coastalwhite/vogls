// Bits of a net that are driven by other bits of the same net propagate through a chain, e.g. the
// state of a scrambler that is unrolled over a net array.
module tb;
   reg  [3:0] din;
   wire [3:0] scr;
   wire [1:0] st [0:4];
   wire [2:0] sh;

   assign st[0] = 2'b00;

   genvar g;
   generate
      for (g = 0; g < 4; g = g + 1) begin : l
         assign scr[g]  = din[g] ^ st[g][1];
         assign st[g+1] = {st[g][0], scr[g]};
      end
   endgenerate

   // Overlapping source and destination bits.
   reg sh_in;
   assign sh[0]   = sh_in;
   assign sh[2:1] = sh[1:0];

   initial begin
      din = 4'b0001;
      sh_in = 1'b1;
      #1;
      $vogls_assert_eq(scr, 4'b0101);
      $vogls_assert_eq(sh, 3'b111);

      din = 4'b0010;
      sh_in = 1'b0;
      #1;
      $vogls_assert_eq(scr, 4'b1010);
      $vogls_assert_eq(sh, 3'b000);
   end
endmodule
