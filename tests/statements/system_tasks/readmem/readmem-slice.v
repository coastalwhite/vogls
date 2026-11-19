// vogls: verify-stdout
module tb;
  // The memory only drives a slice of `bus`, so it is folded into that slice.
  reg [15:0] mem [0:0];
  wire [31:0] bus;
  assign bus[23:8] = mem[0];
  assign bus[31:24] = 8'hab;
  assign bus[7:0] = 8'hcd;

  initial begin
    $readmemh("readmem-slice.hex", mem);
    #1 $display("%h", bus);
  end
endmodule
