module mre_taskarg;
  reg a, b;
  task t(input v, output r);
    begin r = v; end
  endtask
  initial begin
    a = 1'b0; b = 1'b0;
    t(1'b1, a);
    t(1'b0, b);
    $display("a=%b b=%b   (expect a=1 b=0)", a, b);
  end
endmodule