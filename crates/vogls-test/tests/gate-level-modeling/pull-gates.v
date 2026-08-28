module tb;
   reg data, en;
 
   wire free_up, free_dn;
   wire spec_up, spec_dn;
   wire cont_up, cont_dn;
 
   pullup   u0 (free_up);
   pulldown d0 (free_dn);
 
   pullup   u2 (cont_up);
   bufif1   b0 (cont_up, data, en);
 
   pulldown d2 (cont_dn);
   bufif1   b1 (cont_dn, data, en);
 
   task chk;
      input d, e;
      input eu, ed;
      begin
         data = d;
         en   = e;
         #1;
         $vogls_assert_eq(eu, cont_up);
         $vogls_assert_eq(ed, cont_dn);
      end
   endtask
 
   initial begin
      #1;
      $vogls_assert_eq(1'b1, free_up);
      $vogls_assert_eq(1'b0, free_dn);
      $vogls_assert_eq(1'b1, spec_up);
      $vogls_assert_eq(1'b0, spec_dn);
 
      chk(1'b0, 1'b0, 1'b1, 1'b0);
      chk(1'b1, 1'b0, 1'b1, 1'b0);
      chk(1'bx, 1'b0, 1'b1, 1'b0);
      chk(1'bz, 1'b0, 1'b1, 1'b0);
 
      chk(1'b0, 1'b1, 1'b0, 1'b0);
      chk(1'b1, 1'b1, 1'b1, 1'b1);
      chk(1'bx, 1'b1, 1'bx, 1'bx);
      chk(1'bz, 1'b1, 1'bx, 1'bx);
 
      chk(1'b0, 1'bx, 1'bx, 1'bx);
      chk(1'bx, 1'bx, 1'bx, 1'bx);
   end
endmodule
