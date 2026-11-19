// vogls: verify-stdout
// vogls: mode=four-value-logic
module tb;
  // Both planes of a signal under 33 bits are packed in one value.
  reg [3:0] tiny [0:3];
  reg [7:0] mem [0:5];
  reg [69:0] wide [0:1];
  integer i;

  initial begin
    $readmemh("readmem-four-value-tiny.hex", tiny);
    $readmemb("readmem-four-value.bin", mem);
    $readmemh("readmem-four-value-wide.hex", wide);
    for (i = 0; i < 4; i = i + 1) $display("tiny[%0d] = %b", i, tiny[i]);
    for (i = 0; i < 6; i = i + 1) $display("mem[%0d] = %b", i, mem[i]);
    for (i = 0; i < 2; i = i + 1) $display("wide[%0d] = %h", i, wide[i]);
  end
endmodule
