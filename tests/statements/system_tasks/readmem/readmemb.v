// vogls: verify-stdout
module tb;
  // The memory is declared with the highest address first.
  reg [3:0] mem [9:2];
  integer i;

  initial begin
    for (i = 2; i <= 9; i = i + 1) mem[i] = 4'h0;
    // The finish address is below the start address: load in descending order.
    $readmemb("readmemb.bin", mem, 7, 4);
    for (i = 2; i <= 9; i = i + 1) $display("mem[%0d] = %b", i, mem[i]);

    // Only a start address: load until the highest address.
    $readmemb("readmemb.bin", mem, 8);
    for (i = 2; i <= 9; i = i + 1) $display("mem[%0d] = %b", i, mem[i]);
  end
endmodule
