// vogls: fail=elaborate
`timescale 1ns/1ps

module tb;
   wire [3:0] bin, ok;

   function [3:0] gray2bin;
      input [3:0] gray;
      integer k;
      begin
         gray2bin[3] = gray[3];
         for (k = 2; k >= 0; k = k - 1)
           gray2bin[k] = gray2bin[k + 1] ^ gray[k];
      end
   endfunction

   assign bin = gray2bin(4'b1100);
endmodule
