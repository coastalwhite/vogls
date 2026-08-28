module tb;
   reg data, ctrl;

   wire o_nmos, o_pmos, o_rnmos, o_rpmos;

   nmos  g0 (o_nmos,  data, ctrl);
   pmos  g1 (o_pmos,  data, ctrl);
   rnmos g2 (o_rnmos, data, ctrl);
   rpmos g3 (o_rpmos, data, ctrl);

   task chk;
      input d, c;
      input en, ep;
      begin
         data = d;
         ctrl = c;
         #1;
         $vogls_assert_eq(en, o_nmos);
         $vogls_assert_eq(ep, o_pmos);
         $vogls_assert_eq(en, o_rnmos);
         $vogls_assert_eq(ep, o_rpmos);
      end
   endtask

   initial begin
      chk(1'b0, 1'b0, 1'bz, 1'b0);
      chk(1'b0, 1'b1, 1'b0, 1'bz);
      chk(1'b0, 1'bx, 1'bx, 1'bx);
      chk(1'b0, 1'bz, 1'bx, 1'bx);

      chk(1'b1, 1'b0, 1'bz, 1'b1);
      chk(1'b1, 1'b1, 1'b1, 1'bz);
      chk(1'b1, 1'bx, 1'bx, 1'bx);
      chk(1'b1, 1'bz, 1'bx, 1'bx);

      chk(1'bx, 1'b0, 1'bz, 1'bx);
      chk(1'bx, 1'b1, 1'bx, 1'bz);
      chk(1'bx, 1'bx, 1'bx, 1'bx);
      chk(1'bx, 1'bz, 1'bx, 1'bx);

      chk(1'bz, 1'b0, 1'bz, 1'bz);
      chk(1'bz, 1'b1, 1'bz, 1'bz);
      chk(1'bz, 1'bx, 1'bz, 1'bz);
      chk(1'bz, 1'bz, 1'bz, 1'bz);
   end
endmodule
