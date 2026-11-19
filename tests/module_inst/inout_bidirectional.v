// vogls: mode=four-value-logic
//
// An inout port is a bidirectional connection: both the instance and the parent can drive the
// net, and both see the resolved value.
module bus_driver (
   inout [1:0] bus,
   input       en,
   input [1:0] val
);
   assign bus = en ? val : 2'bzz;
endmodule

module tb;
   reg       en_inner, en_outer;
   reg [1:0] val_inner, val_outer;

   wire [1:0] bus;
   wire [3:0] wide;

   assign bus = en_outer ? val_outer : 2'bzz;

   bus_driver inner (.bus(bus), .en(en_inner), .val(val_inner));

   // Two instances on the halves of a wider net.
   bus_driver lo (.bus(wide[1:0]), .en(en_inner), .val(val_inner));
   bus_driver hi (.bus(wide[3:2]), .en(en_outer), .val(val_outer));

   initial begin
      en_inner = 1'b0; val_inner = 2'b00;
      en_outer = 1'b0; val_outer = 2'b00;

      #1;
      $vogls_assert_eq(bus, 2'bzz);
      $vogls_assert_eq(inner.bus, 2'bzz);
      $vogls_assert_eq(wide, 4'bzzzz);

      // Only the instance drives.
      en_inner = 1'b1; val_inner = 2'b10;
      #1;
      $vogls_assert_eq(bus, 2'b10);
      $vogls_assert_eq(inner.bus, 2'b10);
      $vogls_assert_eq(wide, 4'bzz10);

      // Both drive, bit 1 conflicts.
      en_outer = 1'b1; val_outer = 2'b00;
      #1;
      $vogls_assert_eq(bus, 2'bx0);
      $vogls_assert_eq(inner.bus, 2'bx0);
      $vogls_assert_eq(wide, 4'b0010);

      // Only the parent drives, the instance sees it through the port.
      en_inner = 1'b0;
      #1;
      $vogls_assert_eq(bus, 2'b00);
      $vogls_assert_eq(inner.bus, 2'b00);
      $vogls_assert_eq(wide, 4'b00zz);
   end
endmodule
