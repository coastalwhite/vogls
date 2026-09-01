`timescale 1ns/1ps

module tb;
   wire [19:0] arr [0:0];
   wire [19:0] flat;

   assign arr[0] = 20'h06000;
   assign flat   = 20'h06000;

   wire [15:0] slice = arr[0][19 -: 16];

   wire red_inline = |arr[0][19 -: 16];
   wire red_plus   = |arr[0][4  +: 16];
   wire red_flat   = |flat[19 -: 16];
   wire red_range  = |arr[0][19 : 4];
   wire red_stored = |slice;

   initial begin
      #1;
      $vogls_assert_eq(slice, 16'h0600);
      $vogls_assert_eq(red_inline, 1'b1);
      $vogls_assert_eq(red_plus, 1'b1);
      $vogls_assert_eq(red_flat, 1'b1);
      $vogls_assert_eq(red_range, 1'b1);
      $vogls_assert_eq(red_stored, 1'b1);
   end
endmodule
