// vogls: verify-stdout
module x();
   reg [255:0] v;
   initial begin
      v = 256'd0;
      $display("%032h", v);
      v = 256'hDEADBEEF;
      $display("%032h", v);
      $display("%08h", 12'h42);
      $display("%02h", 12'h42);
      $display("%04b", 6'b001011);
   end
endmodule
