// vogls: verify-stdout
module tb;
  reg [7:0] mem [0:15];
  integer i;

  initial begin
    for (i = 0; i < 16; i = i + 1) mem[i] = 8'hee;
    $readmemh("readmemh.hex", mem);
    for (i = 0; i < 16; i = i + 1) $display("mem[%0d] = %h", i, mem[i]);
  end
endmodule
