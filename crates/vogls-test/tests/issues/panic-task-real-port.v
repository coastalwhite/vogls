// vogls: panic
`timescale 1ns/1ps
module panic_task_real_port;
   task take_real;
      input real x;
      begin
      end
   endtask

   task give_real;
      output real x;
      begin x = 1.0; end
   endtask

   initial begin
      take_real(1.0);        // vogls never reaches here: it panics at
      $finish;
   end
endmodule
