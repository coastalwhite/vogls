// vogls: mode=four-value-logic
// Input ports are driven from the parent without being tracked as multidrivers. See
// `module_inst/inout-hierarchy.v` for the same with inout ports.
module tran_ports(input p, input q);
   tran t (p, q);
endmodule

module tb;
   // Exhaustive: one driver per side.
   reg  d1, d2;
   wire s1, s2;
   assign s1 = d1;
   assign s2 = d2;
   tran t0 (s1, s2);

   // Release: the 0 has to come back after the 1 goes away.
   reg  en;
   wire r1, r2, r2_reader;
   assign r1 = 1'b0;
   assign r2 = en ? 1'b1 : 1'bz;
   tran t1 (r1, r2);
   assign r2_reader = r2;

   // Constant drivers on both sides.
   wire k1 = 1'b1;
   wire k2;
   assign k2 = 1'bz;
   tran t2 (k2, k1);

   // Multidriven member.
   reg  ma, mb, mc;
   wire m1, m2;
   assign m1 = ma;
   assign m1 = mb;
   assign m2 = mc;
   tran t3 (m1, m2);

   // Undriven side.
   reg  du;
   wire u1, u2;
   assign u1 = du;
   tran t4 (u1, u2);

   // Chain.
   reg  da, dc;
   wire a, b, c;
   assign a = da;
   assign c = dc;
   tran t5 (a, b);
   tran t6 (b, c);

   // Bit-select terminals.
   reg  [3:0] dv;
   reg  [1:0] dw;
   wire [3:0] v;
   wire [1:0] w;
   assign v = dv;
   assign w = dw;
   tran t7 (v[3], w[0]);

   // Two bits of the same net.
   reg  [2:0] dy;
   wire [2:0] y;
   assign y = dy;
   tran t8 (y[0], y[1]);

   // Across hierarchy.
   reg  dh1, dh2;
   wire h1, h2;
   assign h1 = dh1;
   assign h2 = dh2;
   tran_ports tp (h1, h2);

   // Gate output into a tran'd net.
   reg  gd, ge, dg2;
   wire g1, g2;
   bufif1 g (g1, gd, ge);
   assign g2 = dg2;
   tran t9 (g1, g2);

   task chk_s;
      input x1, x2, e;
      begin
         d1 = x1;
         d2 = x2;
         #1;
         $vogls_assert_eq(e, s1);
         $vogls_assert_eq(e, s2);
      end
   endtask

   task chk_r;
      input x, e;
      begin
         en = x;
         #1;
         $vogls_assert_eq(e, r1);
         $vogls_assert_eq(e, r2);
         $vogls_assert_eq(e, r2_reader);
      end
   endtask

   task chk_m;
      input xa, xb, xc, e;
      begin
         ma = xa;
         mb = xb;
         mc = xc;
         #1;
         $vogls_assert_eq(e, m1);
         $vogls_assert_eq(e, m2);
      end
   endtask

   task chk_u;
      input x;
      begin
         du = x;
         #1;
         $vogls_assert_eq(x, u1);
         $vogls_assert_eq(x, u2);
      end
   endtask

   task chk_c;
      input xa, xc, e;
      begin
         da = xa;
         dc = xc;
         #1;
         $vogls_assert_eq(e, a);
         $vogls_assert_eq(e, b);
         $vogls_assert_eq(e, c);
      end
   endtask

   task chk_v;
      input [3:0] xv;
      input [1:0] xw;
      input [3:0] ev;
      input [1:0] ew;
      begin
         dv = xv;
         dw = xw;
         #1;
         $vogls_assert_eq(ev, v);
         $vogls_assert_eq(ew, w);
      end
   endtask

   task chk_y;
      input [2:0] x;
      input [2:0] e;
      begin
         dy = x;
         #1;
         $vogls_assert_eq(e, y);
      end
   endtask

   task chk_h;
      input x1, x2, e;
      begin
         dh1 = x1;
         dh2 = x2;
         #1;
         $vogls_assert_eq(e, tp.p);
         $vogls_assert_eq(e, tp.q);
      end
   endtask

   task chk_g;
      input xd, xe, x2, e;
      begin
         gd = xd;
         ge = xe;
         dg2 = x2;
         #1;
         $vogls_assert_eq(e, g1);
         $vogls_assert_eq(e, g2);
      end
   endtask

   initial begin
      // Table 4-2.
      chk_s(1'b0, 1'b0, 1'b0);
      chk_s(1'b0, 1'b1, 1'bx);
      chk_s(1'b0, 1'bx, 1'bx);
      chk_s(1'b0, 1'bz, 1'b0);
      chk_s(1'b1, 1'b0, 1'bx);
      chk_s(1'b1, 1'b1, 1'b1);
      chk_s(1'b1, 1'bx, 1'bx);
      chk_s(1'b1, 1'bz, 1'b1);
      chk_s(1'bx, 1'b0, 1'bx);
      chk_s(1'bx, 1'b1, 1'bx);
      chk_s(1'bx, 1'bx, 1'bx);
      chk_s(1'bx, 1'bz, 1'bx);
      chk_s(1'bz, 1'b0, 1'b0);
      chk_s(1'bz, 1'b1, 1'b1);
      chk_s(1'bz, 1'bx, 1'bx);
      chk_s(1'bz, 1'bz, 1'bz);

      chk_r(1'b0, 1'b0);
      chk_r(1'b1, 1'bx);
      chk_r(1'b0, 1'b0);
      chk_r(1'b1, 1'bx);
      chk_r(1'b0, 1'b0);

      $vogls_assert_eq(1'b1, k1);
      $vogls_assert_eq(1'b1, k2);

      chk_m(1'b0, 1'bz, 1'bz, 1'b0);
      chk_m(1'b0, 1'bz, 1'b1, 1'bx);
      chk_m(1'bz, 1'bz, 1'b1, 1'b1);
      chk_m(1'b1, 1'b1, 1'bz, 1'b1);
      chk_m(1'b0, 1'b1, 1'bz, 1'bx);
      chk_m(1'bz, 1'bz, 1'bz, 1'bz);
      chk_m(1'bz, 1'b0, 1'b0, 1'b0);

      chk_u(1'b0);
      chk_u(1'b1);
      chk_u(1'bx);
      chk_u(1'bz);
      chk_u(1'b1);

      chk_c(1'b0, 1'bz, 1'b0);
      chk_c(1'bz, 1'b1, 1'b1);
      chk_c(1'b0, 1'b1, 1'bx);
      chk_c(1'bz, 1'bz, 1'bz);
      chk_c(1'b1, 1'b1, 1'b1);

      chk_v(4'bz101, 2'b10, 4'b0101, 2'b10);
      chk_v(4'b1101, 2'b1z, 4'b1101, 2'b11);
      chk_v(4'b1010, 2'b00, 4'bx010, 2'b0x);
      chk_v(4'bzzzz, 2'bzz, 4'bzzzz, 2'bzz);
      chk_v(4'b0zzz, 2'b1z, 4'b0zzz, 2'b10);

      chk_y(3'b1z0, 3'b100);
      chk_y(3'b01z, 3'b011);
      chk_y(3'bz10, 3'bzxx);
      chk_y(3'b0zz, 3'b0zz);

      chk_h(1'b0, 1'bz, 1'b0);
      chk_h(1'bz, 1'bz, 1'bz);
      chk_h(1'bz, 1'b1, 1'b1);
      chk_h(1'b0, 1'b1, 1'bx);
      chk_h(1'bz, 1'bz, 1'bz);
      chk_h(1'b1, 1'b1, 1'b1);

      chk_g(1'b1, 1'b1, 1'bz, 1'b1);
      chk_g(1'b1, 1'b0, 1'b0, 1'b0);
      chk_g(1'b1, 1'b1, 1'b0, 1'bx);
      chk_g(1'b0, 1'b1, 1'bz, 1'b0);
      chk_g(1'b1, 1'b0, 1'bz, 1'bz);
   end
endmodule
