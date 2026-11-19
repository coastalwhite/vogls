// vogls: mode=four-value-logic
module bottom(inout p, input en, input d);
   assign p = en ? d : 1'bz;
endmodule

module middle(inout p, input en, input d);
   bottom b (p, en, d);
endmodule

module tran_ports(inout p, inout q);
   tran t (p, q);
endmodule

module tb;
   // Two levels of inout ports, with drivers at the top and the bottom.
   reg  en_t, d_t, en_b, d_b;
   wire t;
   assign t = en_t ? d_t : 1'bz;
   middle m (t, en_b, d_b);

   // A `tran` between inout ports.
   reg  dh1, dh2;
   wire h1, h2;
   assign h1 = dh1;
   assign h2 = dh2;
   tran_ports tp (h1, h2);

   task chk_t;
      input et, dt, eb, db, e;
      begin
         en_t = et;
         d_t = dt;
         en_b = eb;
         d_b = db;
         #1;
         $vogls_assert_eq(e, t);
         $vogls_assert_eq(e, m.p);
         $vogls_assert_eq(e, m.b.p);
      end
   endtask

   task chk_h;
      input x1, x2, e;
      begin
         dh1 = x1;
         dh2 = x2;
         #1;
         $vogls_assert_eq(e, h1);
         $vogls_assert_eq(e, h2);
         $vogls_assert_eq(e, tp.p);
         $vogls_assert_eq(e, tp.q);
      end
   endtask

   initial begin
      chk_t(1'b1, 1'b0, 1'b0, 1'b0, 1'b0);
      chk_t(1'b0, 1'b0, 1'b1, 1'b1, 1'b1);
      chk_t(1'b1, 1'b0, 1'b1, 1'b1, 1'bx);
      chk_t(1'b1, 1'b1, 1'b1, 1'b1, 1'b1);
      chk_t(1'b0, 1'b1, 1'b0, 1'b1, 1'bz);
      chk_t(1'b0, 1'b1, 1'b1, 1'b0, 1'b0);
      chk_t(1'b0, 1'b1, 1'b0, 1'b0, 1'bz);

      chk_h(1'b0, 1'bz, 1'b0);
      chk_h(1'bz, 1'bz, 1'bz);
      chk_h(1'bz, 1'b1, 1'b1);
      chk_h(1'b0, 1'b1, 1'bx);
      chk_h(1'bz, 1'bz, 1'bz);
      chk_h(1'b1, 1'b1, 1'b1);
   end
endmodule
