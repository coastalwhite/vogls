// vogls: verify-diagnostics
// vogls: fail=lower
// vogls: backend=bytecode
// vogls: mode=two-value-logic
module tb;
  wire [7:0] mem [0:3];

  initial $readmemh("mem.hex", mem);
endmodule
