// vogls: mode=four-value-logic
// Drivers that are not continuous assignments also take part in resolving a net.
module child(input p);
   assign p = 1'bz;
endmodule

module child_driven(input p, input en);
   assign p = en ? 1'b0 : 1'bz;
endmodule

module tb;
   // A net declaration assignment and a continuous assignment.
   reg  da, db;
   wire w = da;
   assign w = db;

   // The same with an expression, which is not fused.
   wire v = ~da;
   assign v = db;

   // An input port that is also driven inside the module.
   reg  dp, en;
   wire p;
   assign p = dp;
   child_driven u0 (p, en);

   // A fused input port that is also driven inside the module.
   reg  dq;
   child u1 (dq);

   initial begin
      da = 1'b0;
      db = 1'bz;
      #1;
      $vogls_assert_eq(1'b0, w);
      $vogls_assert_eq(1'b1, v);
      db = 1'b1;
      #1;
      $vogls_assert_eq(1'bx, w);
      $vogls_assert_eq(1'b1, v);
      da = 1'bz;
      #1;
      $vogls_assert_eq(1'b1, w);
      $vogls_assert_eq(1'bx, v);

      dp = 1'b1;
      en = 1'b0;
      #1;
      $vogls_assert_eq(1'b1, u0.p);
      en = 1'b1;
      #1;
      $vogls_assert_eq(1'bx, u0.p);
      dp = 1'bz;
      #1;
      $vogls_assert_eq(1'b0, u0.p);

      dq = 1'b1;
      #1;
      $vogls_assert_eq(1'b1, u1.p);
      dq = 1'b0;
      #1;
      $vogls_assert_eq(1'b0, u1.p);
   end
endmodule
