// vogls: verify-diagnostics
// vogls: backend=bytecode
// vogls: mode=two-value-logic
// vogls: fail=lower
module top;
  reg [7:0] r;
  initial r[0+:0] = 1'b0;
endmodule
