`timescale 1ns/1ps
module test;
   function real f_real;
      input real x;
      f_real = x * 2.0;
   endfunction

   localparam real SCALE = 256.0;
   real gold;

   initial begin
      gold = 1.5;
      gold = f_real(gold);
      $vogls_assert_eq($rtoi(SCALE * gold), 768);
   end
endmodule
