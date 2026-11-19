// vogls: fail=execute
module tb;
  reg [7:0] mem [0:3];

  initial begin
    $readmemh("does-not-exist.hex", mem);
    $display("not reached");
  end
endmodule
