// vogls: mode=four-value-logic
//
// Inout ports whose width differs from their connection only connect their common least
// significant bits, and a slice of a bus shared by inout ports of two nested instances.
//
// The expected values match Icarus Verilog.
module leaf(inout [1:0] q, input oe, input [1:0] v, output [1:0] seen);
   assign q = oe ? v : 2'bz;
   assign seen = q;
endmodule

module mid(inout [1:0] p, input oe, input [1:0] v, output [1:0] seen);
   leaf l (p, oe, v, seen);
endmodule

module wide(inout [5:0] w);
   assign w[5] = 1'b1;
   assign w[0] = 1'b0;
endmodule

module narrow(inout [1:0] n);
   assign n = 2'b10;
endmodule

module tb;
   reg  [7:0] drv;
   reg        oe0, oe1;
   reg  [1:0] v0, v1;
   wire [7:0] bus;
   wire [1:0] s0, s1;
   assign bus[7:6] = drv[7:6];
   assign bus[1:0] = drv[1:0];
   mid m0 (bus[3:2], oe0, v0, s0);
   mid m1 (bus[3:2], oe1, v1, s1);

   // The port is wider than its connection: `w[5]` stays inside.
   wire [3:0] wb;
   wide wi (wb);
   // The port is narrower than its connection: the top bits are undriven.
   wire [3:0] nb;
   narrow na (nb);

   initial begin
      drv = 8'b10zzzz01; oe0 = 1'b1; oe1 = 1'b0; v0 = 2'b01; v1 = 2'b10;
      #1;
      $vogls_assert_eq(bus, 8'b10zz0101);
      $vogls_assert_eq(m0.l.q, 2'b01);
      $vogls_assert_eq(s0, 2'b01);
      $vogls_assert_eq(s1, 2'b01);
      $vogls_assert_eq(wb, 4'bzzz0);
      $vogls_assert_eq(nb, 4'bzz10);

      oe1 = 1'b1;
      #1;
      $vogls_assert_eq(bus, 8'b10zzxx01);
      $vogls_assert_eq(s1, 2'bxx);

      oe0 = 1'b0;
      #1;
      $vogls_assert_eq(bus, 8'b10zz1001);
      $vogls_assert_eq(s0, 2'b10);

      oe1 = 1'b0;
      #1;
      $vogls_assert_eq(bus, 8'b10zzzz01);
      $vogls_assert_eq(s0, 2'bzz);
   end
endmodule
