// vogls: verify-stdout
module display_radix;
  reg [11:0]       r12;
  reg  [7:0]       r8;
  reg  [4:0]       r5;
  reg  [3:0]       r4;
  reg signed [7:0] s8;
 
  initial begin
    r12 = 12'h0a5;   // 0000_1010_0101
    r8  = 8'hab;
    r5  = 5'b10110;
    r4  = 4'hf;
    s8  = -1;
 
    $displayb(r12);
    $displayo(r12);
    $displayh(r12);
 
    $displayo(r5);
    $displayh(r5);
    $displayb(r5);
 
    $writeb(r12); $write("|");
    $writeo(r12); $write("|");
    $writeh(r12); $write("\n");
 
    $displayh(r8, r8);
    $displayh(r8,, r8);
 
    $displayh("dec=%d bin=%b oct=%o hex=%h", r12, r12, r12, r12);
 
    $displayb("%0h %0o %0b", r12, r12, r12);
 
    $displayh("100%% done\tok");
 
    $displayh(14'bx01010);
    $displayh(12'b001xxx101x01);
    $displayo(12'b001xxx101x01);
    $displayb(12'b001xxx101x01);
 
    $displayh(8'bzzzz0011);
    $displayh(8'bzz100011);
    $displayh(8'bzzx00011);
 
    $displayh("[%d][%d][%d][%d][%d]",
              8'bxxxxxxxx, 8'bzzzzzzzz, 8'bxxxx0011, 8'bzzzz0011, 8'bxxxxzzzz);
 
    $displayb({4'ha, 4'h5});
    $displayh(r4 + r4);
 
    $displayh("%d", s8);
    $displayh(s8);
 
    $displayb;
    $writeh;
    $writeo("done\n");
  end
endmodule
