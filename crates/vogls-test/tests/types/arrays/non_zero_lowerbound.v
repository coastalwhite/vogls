`timescale 1ns/1ps
module tb;
   wire [7:0]  w [1:3];
   wire [7:0]  z [0:3];
   reg [7:0]   m [1:3];
   wire [12:0] sum;
   assign w[1] = 8'h11; assign w[2] = 8'h22; assign w[3] = 8'h33;
   assign z[0] = 8'ha0; assign z[1] = 8'ha1;
   assign z[2] = 8'ha2; assign z[3] = 8'ha3;
   assign sum = {5'b0, 8'hFF} + {5'b0, w[3]};
   initial begin
      m[1] = 8'hd1; m[2] = 8'hd2; m[3] = 8'hd3;
      #1;
      $vogls_assert_eq(w[1], 8'h11); $vogls_assert_eq(w[2], 8'h22); $vogls_assert_eq(w[3], 8'h33);
      $vogls_assert_eq(z[0], 8'ha0); $vogls_assert_eq(z[1], 8'ha1); $vogls_assert_eq(z[2], 8'ha2); $vogls_assert_eq(z[3], 8'ha3);
      $vogls_assert_eq(m[1], 8'hd1); $vogls_assert_eq(m[2], 8'hd2); $vogls_assert_eq(m[3], 8'hd3);
      $vogls_assert_eq(sum, 13'h132);
      $finish;
   end
endmodule
