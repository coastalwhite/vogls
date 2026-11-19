// vogls: mode=four-value-logic
module tristate(inout p, input en, input d);
   assign p = en ? d : 1'bz;
endmodule

module driver(inout p, input d);
   assign p = d;
endmodule

module tb;
   // Bidirectional bus: a tri-state driver on both sides.
   reg  en_p, dp, en_c, dc;
   wire bus;
   assign bus = en_p ? dp : 1'bz;
   tristate u0 (bus, en_c, dc);

   // Release: only the child drives.
   reg  en_r, dr;
   wire r;
   tristate u1 (r, en_r, dr);

   // A driver inside the child and a driver outside.
   reg  d_out, d_in;
   wire s;
   assign s = d_out;
   driver u2 (s, d_in);

   reg pv, cv, e;
   integer i;

   task chk_r;
      input en, d, e;
      begin
         en_r = en;
         dr = d;
         #1;
         $vogls_assert_eq(e, r);
         $vogls_assert_eq(e, u1.p);
      end
   endtask

   task chk_s;
      input x_out, x_in, e;
      begin
         d_out = x_out;
         d_in = x_in;
         #1;
         $vogls_assert_eq(e, s);
         $vogls_assert_eq(e, u2.p);
      end
   endtask

   initial begin
      for (i = 0; i < 36; i = i + 1) begin
         en_p = i[0];
         en_c = i[1];
         dp = i[3:2] == 2 ? 1'bx : i[2];
         dc = i[5:4] == 2 ? 1'bx : i[4];
         if (i[3:2] == 3 || i[5:4] == 3) begin
            // Skip: these encode nothing new.
         end else begin
            #1;
            pv = en_p ? dp : 1'bz;
            cv = en_c ? dc : 1'bz;
            e = pv === 1'bz ? cv : cv === 1'bz ? pv : pv === cv ? pv : 1'bx;
            $vogls_assert_eq(e, bus);
            $vogls_assert_eq(e, u0.p);
         end
      end

      chk_r(1'b1, 1'b1, 1'b1);
      chk_r(1'b0, 1'b1, 1'bz);
      chk_r(1'b1, 1'b0, 1'b0);
      chk_r(1'b0, 1'b0, 1'bz);
      chk_r(1'b1, 1'b1, 1'b1);

      chk_s(1'b0, 1'b0, 1'b0);
      chk_s(1'b0, 1'b1, 1'bx);
      chk_s(1'b0, 1'bz, 1'b0);
      chk_s(1'b1, 1'bz, 1'b1);
      chk_s(1'bz, 1'b1, 1'b1);
      chk_s(1'bz, 1'b0, 1'b0);
      chk_s(1'bz, 1'bz, 1'bz);
      chk_s(1'bx, 1'bz, 1'bx);
      chk_s(1'b1, 1'b1, 1'b1);
   end
endmodule
