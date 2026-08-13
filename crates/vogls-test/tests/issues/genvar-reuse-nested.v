// vogls: fail=elaborate
`timescale 1ns/1ps

module dut (output wire [3:0] e);

   genvar i, j;

   generate
      // j used as a top-level generate-for index first
      for (j = 0; j < 4; j = j + 1) begin : g_edge
         assign e[j] = 1'b1;
      end

      // then reused as the index of a NESTED generate-for: vogls rejects this
      for (i = 0; i < 4; i = i + 1) begin : g_row
         for (j = 0; j < 4; j = j + 1) begin : g_col
            wire t;
            assign t = 1'b1;
         end
      end
   endgenerate

endmodule

module tb;

   wire [3:0] e;

   dut u_dut (.e(e));

   initial begin
      #1;
      $display("e = %0h (expected f)", e);
      $finish;
   end

endmodule
