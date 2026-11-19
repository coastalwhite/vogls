// vogls: verify-stdout
module tb;
  // Words that cross 64-bit boundaries, and words wider than 64 bits.
  reg [36:0] narrow [0:5];
  reg [99:0] wide [0:2];
  integer i;

  initial begin
    $readmemh("readmem-wide-narrow.hex", narrow);
    $readmemh("readmem-wide-wide.hex", wide);
    for (i = 0; i < 6; i = i + 1) $display("narrow[%0d] = %h", i, narrow[i]);
    for (i = 0; i < 3; i = i + 1) $display("wide[%0d] = %h", i, wide[i]);
  end
endmodule
