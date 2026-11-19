// vogls: mode=four-value-logic
//
// `tran` switches that form a loop, a gate output in a `tran` closure and a closure that reaches
// through a `tran` in a submodule.
//
// The expected values match Icarus Verilog.
module tran_ports(inout x, inout y);
   tran t (x, y);
endmodule

module tb;
   // A loop of three switches, driven at one member.
   reg  vl;
   wire l1, l2, l3;
   tran t0 (l1, l2);
   tran t1 (l2, l3);
   tran t2 (l3, l1);
   assign l2 = vl;

   // A tri-state gate on one side, a constant on the other.
   reg  gi, ge;
   wire g1, g2;
   bufif1 (g1, gi, ge);
   tran t3 (g1, g2);
   assign g2 = 1'b0;

   // A chain with tri-state drivers at both ends, through a submodule.
   reg  ea, va, ec, vc;
   wire ta, tb_, tc;
   assign ta = ea ? va : 1'bz;
   assign tc = ec ? vc : 1'bz;
   tran_ports tp (ta, tb_);
   tran t4 (tb_, tc);

   initial begin
      vl = 1'b1; gi = 1'b1; ge = 1'b0;
      ea = 1'b1; va = 1'b0; ec = 1'b0; vc = 1'b1;
      #1;
      $vogls_assert_eq({l1, l2, l3}, 3'b111);
      $vogls_assert_eq({g1, g2}, 2'b00);
      $vogls_assert_eq({ta, tb_, tc}, 3'b000);

      vl = 1'b0; ge = 1'b1; ec = 1'b1;
      #1;
      $vogls_assert_eq({l1, l2, l3}, 3'b000);
      $vogls_assert_eq({g1, g2}, 2'bxx);
      $vogls_assert_eq({ta, tb_, tc}, 3'bxxx);

      gi = 1'b0; ea = 1'b0;
      #1;
      $vogls_assert_eq({g1, g2}, 2'b00);
      $vogls_assert_eq({ta, tb_, tc}, 3'b111);

      ec = 1'b0;
      #1;
      $vogls_assert_eq({ta, tb_, tc}, 3'bzzz);
   end
endmodule
