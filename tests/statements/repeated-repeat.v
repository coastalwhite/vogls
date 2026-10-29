// vogls: verify-stdout
module t;
  reg clk = 0; always #5 clk = ~clk;
  integer pass;
  initial begin
    for (pass = 0; pass < 3; pass = pass + 1) begin
      repeat (4) @(posedge clk);
      $display("pass %0d at %0t", pass, $time);
    end
    $finish;
  end
endmodule
