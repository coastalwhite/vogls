module test;
   reg r;
   initial begin
      r = ($random % 8 == 0);
      $finish;
   end
endmodule
