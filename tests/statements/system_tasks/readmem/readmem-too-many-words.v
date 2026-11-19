// vogls: verify-stdout
module tb;
  reg [7:0] before;
  reg [7:0] mem [0:3];
  reg [7:0] after;
  integer i;

  initial begin
    before = 8'h11;
    after = 8'h22;
    // The words that do not fit are dropped with a warning.
    $readmemh("readmem-too-many-words.hex", mem);
    for (i = 0; i < 4; i = i + 1) $display("mem[%0d] = %h", i, mem[i]);
    $display("before = %h, after = %h", before, after);
  end
endmodule
