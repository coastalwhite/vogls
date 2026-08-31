`timescale 1ns/1ps
module tb;
   function [3:0] f_two_loops;
      integer i;
      reg [3:0] s;
      begin
         s = 0;
         for (i = 0; i < 4; i = i + 1) s = s + 1;
         for (i = 0; i < 4; i = i + 1) begin s = s + 1; end
         f_two_loops = s;
      end
   endfunction
endmodule
