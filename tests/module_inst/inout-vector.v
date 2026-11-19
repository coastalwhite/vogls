// vogls: mode=four-value-logic
module tristate8(inout [7:0] p, input en, input [7:0] d);
   assign p = en ? d : 8'bz;
endmodule

module tristate4(inout [3:0] p, input en, input [3:0] d);
   assign p = en ? d : 4'bz;
endmodule

module driver2(inout [1:0] p, input [1:0] d);
   assign p = d;
endmodule

module tb;
   // Whole vector.
   reg        en_a;
   reg  [7:0] da, pa;
   wire [7:0] a;
   assign a = pa;
   tristate8 u0 (a, en_a, da);

   // Part-select.
   reg        en_b;
   reg  [3:0] db;
   reg  [7:0] pb;
   wire [7:0] b;
   assign b = pb;
   tristate4 u1 (b[7:4], en_b, db);

   // Concatenation.
   reg  [1:0] dc;
   reg        pc0, pc1;
   wire       c0, c1;
   assign c0 = pc0;
   assign c1 = pc1;
   driver2 u2 ({c1, c0}, dc);

   // Instance array.
   reg  [3:0]  en_d;
   reg  [15:0] dd, pd;
   wire [15:0] d;
   assign d = pd;
   tristate4 u3[3:0] (d, en_d, dd);

   initial begin
      pa = 8'bz;
      da = 8'b1010_0110;
      en_a = 1'b1;
      #1;
      $vogls_assert_eq(8'b1010_0110, a);
      $vogls_assert_eq(8'b1010_0110, u0.p);
      pa = 8'b1111_zzzz;
      #1;
      $vogls_assert_eq(8'b1x1x_0110, a);
      $vogls_assert_eq(8'b1x1x_0110, u0.p);
      en_a = 1'b0;
      #1;
      $vogls_assert_eq(8'b1111_zzzz, a);
      $vogls_assert_eq(8'b1111_zzzz, u0.p);

      pb = 8'bzzzz_1010;
      db = 4'b0110;
      en_b = 1'b1;
      #1;
      $vogls_assert_eq(8'b0110_1010, b);
      $vogls_assert_eq(4'b0110, u1.p);
      en_b = 1'b0;
      #1;
      $vogls_assert_eq(8'bzzzz_1010, b);
      $vogls_assert_eq(4'bzzzz, u1.p);
      pb = 8'b10zz_1010;
      en_b = 1'b1;
      #1;
      $vogls_assert_eq(8'bxx10_1010, b);
      $vogls_assert_eq(4'bxx10, u1.p);

      pc0 = 1'bz;
      pc1 = 1'bz;
      dc = 2'b10;
      #1;
      $vogls_assert_eq(1'b1, c1);
      $vogls_assert_eq(1'b0, c0);
      $vogls_assert_eq(2'b10, u2.p);
      pc0 = 1'b1;
      #1;
      $vogls_assert_eq(1'b1, c1);
      $vogls_assert_eq(1'bx, c0);
      $vogls_assert_eq(2'b1x, u2.p);

      pd = 16'hzz3z;
      dd = 16'h8421;
      en_d = 4'b0101;
      #1;
      $vogls_assert_eq(16'bzzzz_0100_0011_0001, d);
      en_d = 4'b1111;
      #1;
      $vogls_assert_eq(16'b1000_0100_001x_0001, d);
      en_d = 4'b0000;
      #1;
      $vogls_assert_eq(16'bzzzz_zzzz_0011_zzzz, d);
   end
endmodule
