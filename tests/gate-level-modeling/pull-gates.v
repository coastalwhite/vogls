module tb;
   reg data, en;
 
   wire free_up, free_dn;
 
   pullup   u0 (free_up);
   pulldown d0 (free_dn);
 
   initial begin
      #1;
      $vogls_assert_eq(1'b1, free_up);
      $vogls_assert_eq(1'b0, free_dn);
   end
endmodule
