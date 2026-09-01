`timescale 1ns/1ps

module tb;
   function integer fA;
      input integer a, b;
      fA = a*100 + b;
   endfunction

   function integer fB;
      input integer a;
      input integer b;
      fB = a*100 + b;
   endfunction

   function integer fC;
      input integer a, b, c;
      fC = a*100 + b*10 + c;
   endfunction

   initial begin
      $vogls_assert_eq(fA(3, 7), 307);
      $vogls_assert_eq(fB(3, 7), 307);
      $vogls_assert_eq(fC(1, 2, 3), 123);
   end
endmodule
