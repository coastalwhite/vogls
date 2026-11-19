// vogls: mode=four-value-logic
//
// `tran` switches whose connections only partially overlap or that connect a signal to itself
// cannot be resolved as whole slices and are resolved bit by bit instead.
module tb;
   reg [1:0] ea, va;
   reg       ec, vc;
   reg       ed, vd;
   reg       es, vs;

   wire [1:0] a, c;
   wire       d;
   wire [3:0] s;

   assign a[0] = ea[0] ? va[0] : 1'bz;
   assign a[1] = ea[1] ? va[1] : 1'bz;
   assign c[1] = ec ? vc : 1'bz;
   assign d    = ed ? vd : 1'bz;
   assign s[0] = es ? vs : 1'bz;

   // Partially overlapping: bit 0 of `a` and `c` is also connected to `d`.
   //   {a[0], c[0], d}  and  {a[1], c[1]}
   tran t0 (a, c);
   tran t1 (a[0], d);

   // Self-connected: {s[0], s[2]} and {s[1], s[3]}.
   tran t2 (s[1:0], s[3:2]);

   initial begin
      ea = 2'b00; va = 2'b00;
      ec = 1'b0;  vc = 1'b0;
      ed = 1'b0;  vd = 1'b0;
      es = 1'b0;  vs = 1'b0;

      // Nothing is driven.
      #1;
      $vogls_assert_eq(a, 2'bzz);
      $vogls_assert_eq(c, 2'bzz);
      $vogls_assert_eq(d, 1'bz);
      $vogls_assert_eq(s, 4'bzzzz);

      // `d` drives bit 0 of `a` and `c`, but not bit 1.
      ed = 1'b1; vd = 1'b1;
      #1;
      $vogls_assert_eq(a, 2'bz1);
      $vogls_assert_eq(c, 2'bz1);
      $vogls_assert_eq(d, 1'b1);

      // `c[1]` drives bit 1 of `a`.
      ec = 1'b1; vc = 1'b0;
      #1;
      $vogls_assert_eq(a, 2'b01);
      $vogls_assert_eq(c, 2'b01);
      $vogls_assert_eq(d, 1'b1);

      // `a[0]` conflicts with `d`.
      ea = 2'b01; va = 2'b00;
      #1;
      $vogls_assert_eq(a, 2'b0x);
      $vogls_assert_eq(c, 2'b0x);
      $vogls_assert_eq(d, 1'bx);

      // `d` releases, so `a[0]` wins.
      ed = 1'b0;
      #1;
      $vogls_assert_eq(a, 2'b00);
      $vogls_assert_eq(c, 2'b00);
      $vogls_assert_eq(d, 1'b0);

      // `s[0]` drives `s[2]`, but not the odd bits.
      es = 1'b1; vs = 1'b1;
      #1;
      $vogls_assert_eq(s, 4'bz1z1);
   end
endmodule
