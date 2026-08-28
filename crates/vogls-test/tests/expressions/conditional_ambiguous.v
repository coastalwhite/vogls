module tb;
   reg         c;
   reg         t1, f1;
   reg   [7:0] t8, f8;
   reg  [99:0] tw, fw;

   wire        r1 = c ? t1 : f1;
   wire  [7:0] r8 = c ? t8 : f8;
   wire [99:0] rw = c ? tw : fw;

   task chk1;
      input cin, tin, fin, e;
      begin
         c = cin; t1 = tin; f1 = fin;
         #1 $vogls_assert_eq(e, r1);
      end
   endtask

   task chk8;
      input cin;
      input [7:0] tin, fin, e;
      begin
         c = cin; t8 = tin; f8 = fin;
         #1 $vogls_assert_eq(e, r8);
      end
   endtask

   task chkw;
      input cin;
      input [99:0] tin, fin, e;
      begin
         c = cin; tw = tin; fw = fin;
         #1 $vogls_assert_eq(e, rw);
      end
   endtask

   initial begin
      chk1(1'b1, 1'b1, 1'b0, 1'b1);
      chk1(1'b0, 1'b1, 1'b0, 1'b0);
      chk8(1'b1, 8'h55, 8'h33, 8'h55);
      chk8(1'b0, 8'h55, 8'h33, 8'h33);
      chkw(1'b1, 100'h5555555555555555555555555,
                 100'h3333333333333333333333333,
                 100'h5555555555555555555555555);
      chkw(1'b0, 100'h5555555555555555555555555,
                 100'h3333333333333333333333333,
                 100'h3333333333333333333333333);

`ifndef __VOGLS__TWO_VALUE_LOGIC
      chk1(1'b1, 1'bz, 1'b0, 1'bz);
      chk1(1'b0, 1'b1, 1'bz, 1'bz);
      chk1(1'b1, 1'bx, 1'b1, 1'bx);

      chk1(1'bx, 1'b0, 1'b0, 1'b0);
      chk1(1'bx, 1'b1, 1'b1, 1'b1);
      chk1(1'bz, 1'b0, 1'b0, 1'b0);
      chk1(1'bz, 1'b1, 1'b1, 1'b1);

      chk1(1'bx, 1'b0, 1'b1, 1'bx);
      chk1(1'bx, 1'b1, 1'b0, 1'bx);
      chk1(1'bx, 1'bz, 1'bz, 1'bx);
      chk1(1'bx, 1'bx, 1'bx, 1'bx);
      chk1(1'bx, 1'b1, 1'bz, 1'bx);
      chk1(1'bz, 1'bz, 1'bz, 1'bx);

      chk8(1'bx, 8'h55, 8'h33, {2{4'b0xx1}});
      chk8(1'bz, 8'h55, 8'h33, {2{4'b0xx1}});
      chk8(1'bx, 8'h55, 8'h55, 8'h55);
      chk8(1'bx, 8'h00, 8'hFF, 8'bxxxxxxxx);

      chkw(1'bx, 100'h5555555555555555555555555,
                 100'h3333333333333333333333333,
                 {25{4'b0xx1}});
      chkw(1'bz, 100'h5555555555555555555555555,
                 100'h3333333333333333333333333,
                 {25{4'b0xx1}});
      chkw(1'bx, 100'h5555555555555555555555555,
                 100'h5555555555555555555555555,
                 100'h5555555555555555555555555);
      chkw(1'bx, {36'h0, 64'hFFFFFFFFFFFFFFFF},
                 {36'hF, 64'hFFFFFFFFFFFFFFFF},
                 {32'b0, 4'bxxxx, 64'hFFFFFFFFFFFFFFFF});
`endif
   end
endmodule
