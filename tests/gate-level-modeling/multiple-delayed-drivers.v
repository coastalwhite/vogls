// vogls: mode=four-value-logic
//
// The output of a gate with a delay is `x` until the gate first drives it. With several drivers,
// each delayed output is resolved with the others, so the net is `x` too, not `z`.
//
// The expected values match Icarus Verilog.
module tb;
   reg a, e, e2;
   wire w1, w2;

   // Two delayed drivers.
   bufif1 #5 (w1, a, e);
   bufif1 #5 (w1, a, e);

   // A delayed driver and a disabled tristate driver.
   bufif1 #5 (w2, a, e);
   bufif1    (w2, a, e2);

   initial begin
      a = 1; e = 1; e2 = 0;
      #1;
      $vogls_assert_eq(w1, 1'bx);
      $vogls_assert_eq(w2, 1'bx);
      #5;
      $vogls_assert_eq(w1, 1'b1);
      $vogls_assert_eq(w2, 1'b1);
   end
endmodule
