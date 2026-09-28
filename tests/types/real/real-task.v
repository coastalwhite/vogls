`timescale 1ns/1ps
module tb;
   real out;
   task take_real;
      input real x;
      begin
      end
   endtask

   task give_real;
      output real x;
      begin x = 1337.0; end
   endtask

   initial begin
      take_real(1.0);
	  give_real(out);
      $vogls_assert_eq(out, 1337.0);
   end
endmodule
