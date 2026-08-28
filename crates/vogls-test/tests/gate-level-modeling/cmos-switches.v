module tb;
   reg data, nctrl, pctrl;

   wire o_cmos, o_rcmos;

   cmos  g0 (o_cmos,  data, nctrl, pctrl);
   rcmos g1 (o_rcmos, data, nctrl, pctrl);

   task chk;
      input d, nc, pc;
      input e;
      begin
         data  = d;
         nctrl = nc;
         pctrl = pc;
         #1;
         $vogls_assert_eq(e, o_cmos);
         $vogls_assert_eq(e, o_rcmos);
      end
   endtask

   initial begin
      chk(1'b0, 1'b0, 1'b0, 1'b0);
      chk(1'b0, 1'b0, 1'b1, 1'bz);
      chk(1'b0, 1'b0, 1'bx, 1'bx);
      chk(1'b0, 1'b0, 1'bz, 1'bx);
      chk(1'b0, 1'b1, 1'b0, 1'b0);
      chk(1'b0, 1'b1, 1'b1, 1'b0);
      chk(1'b0, 1'b1, 1'bx, 1'b0);
      chk(1'b0, 1'b1, 1'bz, 1'b0);
      chk(1'b0, 1'bx, 1'b0, 1'b0);
      chk(1'b0, 1'bx, 1'b1, 1'bx);
      chk(1'b0, 1'bx, 1'bx, 1'bx);
      chk(1'b0, 1'bx, 1'bz, 1'bx);
      chk(1'b0, 1'bz, 1'b0, 1'b0);
      chk(1'b0, 1'bz, 1'b1, 1'bx);
      chk(1'b0, 1'bz, 1'bx, 1'bx);
      chk(1'b0, 1'bz, 1'bz, 1'bx);

      chk(1'b1, 1'b0, 1'b0, 1'b1);
      chk(1'b1, 1'b0, 1'b1, 1'bz);
      chk(1'b1, 1'b0, 1'bx, 1'bx);
      chk(1'b1, 1'b0, 1'bz, 1'bx);
      chk(1'b1, 1'b1, 1'b0, 1'b1);
      chk(1'b1, 1'b1, 1'b1, 1'b1);
      chk(1'b1, 1'b1, 1'bx, 1'b1);
      chk(1'b1, 1'b1, 1'bz, 1'b1);
      chk(1'b1, 1'bx, 1'b0, 1'b1);
      chk(1'b1, 1'bx, 1'b1, 1'bx);
      chk(1'b1, 1'bx, 1'bx, 1'bx);
      chk(1'b1, 1'bx, 1'bz, 1'bx);
      chk(1'b1, 1'bz, 1'b0, 1'b1);
      chk(1'b1, 1'bz, 1'b1, 1'bx);
      chk(1'b1, 1'bz, 1'bx, 1'bx);
      chk(1'b1, 1'bz, 1'bz, 1'bx);

      chk(1'bx, 1'b0, 1'b0, 1'bx);
      chk(1'bx, 1'b0, 1'b1, 1'bz);
      chk(1'bx, 1'b0, 1'bx, 1'bx);
      chk(1'bx, 1'b0, 1'bz, 1'bx);
      chk(1'bx, 1'b1, 1'b0, 1'bx);
      chk(1'bx, 1'b1, 1'b1, 1'bx);
      chk(1'bx, 1'b1, 1'bx, 1'bx);
      chk(1'bx, 1'b1, 1'bz, 1'bx);
      chk(1'bx, 1'bx, 1'b0, 1'bx);
      chk(1'bx, 1'bx, 1'b1, 1'bx);
      chk(1'bx, 1'bx, 1'bx, 1'bx);
      chk(1'bx, 1'bx, 1'bz, 1'bx);
      chk(1'bx, 1'bz, 1'b0, 1'bx);
      chk(1'bx, 1'bz, 1'b1, 1'bx);
      chk(1'bx, 1'bz, 1'bx, 1'bx);
      chk(1'bx, 1'bz, 1'bz, 1'bx);

      chk(1'bz, 1'b0, 1'b0, 1'bz);
      chk(1'bz, 1'b0, 1'b1, 1'bz);
      chk(1'bz, 1'b0, 1'bx, 1'bz);
      chk(1'bz, 1'b0, 1'bz, 1'bz);
      chk(1'bz, 1'b1, 1'b0, 1'bz);
      chk(1'bz, 1'b1, 1'b1, 1'bz);
      chk(1'bz, 1'b1, 1'bx, 1'bz);
      chk(1'bz, 1'b1, 1'bz, 1'bz);
      chk(1'bz, 1'bx, 1'b0, 1'bz);
      chk(1'bz, 1'bx, 1'b1, 1'bz);
      chk(1'bz, 1'bx, 1'bx, 1'bz);
      chk(1'bz, 1'bx, 1'bz, 1'bz);
      chk(1'bz, 1'bz, 1'b0, 1'bz);
      chk(1'bz, 1'bz, 1'b1, 1'bz);
      chk(1'bz, 1'bz, 1'bx, 1'bz);
      chk(1'bz, 1'bz, 1'bz, 1'bz);
   end
endmodule
