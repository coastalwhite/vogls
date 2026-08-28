module tb;
   reg data, ctrl;

   wire o_bufif0, o_bufif1, o_notif0, o_notif1;

   bufif0 g0 (o_bufif0, data, ctrl);
   bufif1 g1 (o_bufif1, data, ctrl);
   notif0 g2 (o_notif0, data, ctrl);
   notif1 g3 (o_notif1, data, ctrl);

   task chk;
      input d, c;
      input e0, e1, e2, e3;
      begin
         data = d;
         ctrl = c;
         #1;
         $vogls_assert_eq(e0, o_bufif0);
         $vogls_assert_eq(e1, o_bufif1);
         $vogls_assert_eq(e2, o_notif0);
         $vogls_assert_eq(e3, o_notif1);
      end
   endtask

   initial begin
      chk(1'b0, 1'b0, 1'b0, 1'bz, 1'b1, 1'bz);
      chk(1'b0, 1'b1, 1'bz, 1'b0, 1'bz, 1'b1);
      chk(1'b0, 1'bx, 1'bx, 1'bx, 1'bx, 1'bx);
      chk(1'b0, 1'bz, 1'bx, 1'bx, 1'bx, 1'bx);

      chk(1'b1, 1'b0, 1'b1, 1'bz, 1'b0, 1'bz);
      chk(1'b1, 1'b1, 1'bz, 1'b1, 1'bz, 1'b0);
      chk(1'b1, 1'bx, 1'bx, 1'bx, 1'bx, 1'bx);
      chk(1'b1, 1'bz, 1'bx, 1'bx, 1'bx, 1'bx);

      chk(1'bx, 1'b0, 1'bx, 1'bz, 1'bx, 1'bz);
      chk(1'bx, 1'b1, 1'bz, 1'bx, 1'bz, 1'bx);
      chk(1'bx, 1'bx, 1'bx, 1'bx, 1'bx, 1'bx);
      chk(1'bx, 1'bz, 1'bx, 1'bx, 1'bx, 1'bx);

      chk(1'bz, 1'b0, 1'bx, 1'bz, 1'bx, 1'bz);
      chk(1'bz, 1'b1, 1'bz, 1'bx, 1'bz, 1'bx);
      chk(1'bz, 1'bx, 1'bx, 1'bx, 1'bx, 1'bx);
      chk(1'bz, 1'bz, 1'bx, 1'bx, 1'bx, 1'bx);
   end
endmodule
