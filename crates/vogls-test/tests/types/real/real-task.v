`timescale 1ns/1ps
module panic_task_real_port;
   task take_real;
      input real x;
      begin
      end
   endtask

   task give_real;
      input _dummy;
      output real x;
      begin x = 1337.0; end
   endtask

   initial begin
      take_real(1.0);
      $vogls_assert_eq(give_real(0), 1337.0);
   end
endmodule
